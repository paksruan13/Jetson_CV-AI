"""MERLIN Core Speech Pipeline: Audio"""

import os
os.environ["PYTORCH_CUDA_ALLOC_CONF"] = 'backend:native, expandable_segments:True'

import sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).parent.parent))
import torch
import sounddevice as sd
import gc
import numpy as np
import scipy.signal
import time
from faster_whisper import WhisperModel
import webrtcvad
from reasoning_engine.llm_client import LLMClient
from speech_synthesis.tts_engine import TTSEngine

# import rust audio filters
try:
    from merlin_audio import PyNoiseGate, PyNormalizer
    RUST_FILTERS = True
    print(f"Rust Filters Available")
except ImportError as e:
    RUST_FILTERS = False
    print(f"Rust filters Unavailable: {e}")

class VoiceBrain:
    def __init__(self):
        print("Initializing MERLIN's Voice Brain...")
        # Load whisper once, keep loaded
        self._whisper = WhisperModel(
            "tiny",
            device = "cuda",
            compute_type = "float16"
        )
        print("Model loaded")

        #Load WebRTC VAD
        self.vad = webrtcvad.Vad(3)
        print("VAD initialized")

        #Load Rust audio filters
        if RUST_FILTERS:
            print("Loading Rust Audio filters...")
            self.noise_gate = PyNoiseGate (
                threshold_db = -45.0,
                attack_ms = 5.0,
                release_ms = 100.0,
                sample_rate = 16000.0
            )
            self.normalizer = PyNormalizer (
                target_level_db = -20.0,
                window_ms = 200.0,
                sample_rate = 16000.0
            )
            print("Audio Filters Ready")
        else:
            self.noise_gate = None
            self.normalizer = None

        print("Loading LLM Client...")
        self.llm = LLMClient()
        print("Client Ready")

        print("Loading TTSEngine...")
        self.tts = TTSEngine()
        print("TTS Ready")

        self.conversation_history = []
        self.user_name = None

        self.wake_words = ["merlin", "hey merlin"]

        self.conversation_history = []
        self.max_history = 10

        #Audio Setting
        self.sample_rate = 16000
        self.chunk_duration = 0.02
        self.frame_samples = int(self.sample_rate * self.chunk_duration)
        self.silence_duration = 2.0 #2s silence to indicate end of speech

        print(f"Wake words : {self.wake_words}")

    def apply_filters(self, audio_f32):
        """Apply rust audio to 160khz audio chunk"""
        if not RUST_FILTERS or self.noise_gate is None:
            return audio_f32
        
        # Apply noise gate
        filtered_bytes = self.noise_gate.process(audio_f32.tobytes())

        # Normalize
        normalized_bytes = self.normalizer.process(filtered_bytes)

        # Convert back to f32 array
        result = np.frombuffer(normalized_bytes, dtype = np.float32)

        return result
    def is_speech(self, audio_chunk):
        "WebRTC VAD for speech detection"
        if len(audio_chunk) != self.frame_samples:
            return False
        pcm = (audio_chunk * 32768).astype(np.int16).tobytes()
        try:
            return self.vad.is_speech(pcm, self.sample_rate)
        except:
            return False
    
    def listen_with_vad(self):
        print("\nWaiting for speech", end="", flush=True)
        device_info = sd.query_devices(0, 'input')
        mic_rate = int(device_info['default_samplerate'])

        chunks = []
        silence_chunks = 0
        max_silence_chunks = int(self.silence_duration / self.chunk_duration)
        speech_started = False
        mic_blocksize = int(mic_rate * self.chunk_duration)

        def callback(indata, frames, time_info, status):
            nonlocal silence_chunks, speech_started
            chunk = indata.flatten()
            #resample chunk to 16khz for VAD...mic is 44.1kHz
            num_samples = int(len(chunk) * self.sample_rate / mic_rate)
            chunk_16k = scipy.signal.resample(chunk, num_samples).astype(np.float32)

            # Apply Rust filters after resampling, Before VAD
            filtered_chunk = self.apply_filters(chunk_16k)
            #check if speech
            if self.is_speech(filtered_chunk):
                if not speech_started:
                    if not hasattr(callback, "speech_frames"):
                        callback.speech_frames = 0
                    callback.speech_frames += 1
                    if callback.speech_frames >= 3:
                        print("\n Speech deteced!", end="", flush=True)
                        speech_started = True
                        callback.speech_frames = 0
                else:
                    chunks.append(filtered_chunk)
                    silence_chunks = 0
                    print("Now Speaking", end="", flush=True)
            else:
                if not speech_started and hasattr(callback, "speech_frames"):
                    callback.speech_frames = 0
                if speech_started:
                    silence_chunks += 1
                    print(".", end="", flush=True)

        stream = sd.InputStream(
            callback = callback,
            channels = 1,
            samplerate = mic_rate,
            blocksize = mic_blocksize,
            dtype = np.float32,
            device = 0
        )

        with stream:
            #wait for speech & silence
            while True:
                if speech_started and silence_chunks >= max_silence_chunks:
                    break
                if len(chunks) > 300:
                    break
                sd.sleep(100)
        print()

        if len(chunks) == 0:
            return None
        
        #Combine chunks
        audio = np.concatenate(chunks).flatten()
        return audio.astype(np.float32)
    
    def transcribe(self, audio):
        print("Transcribing...")
        start = time.time()
        segments, info = self._whisper.transcribe(
            audio, 
            language = "en",
            beam_size = 5
            )
        text = " ".join([s.text for s in segments])
        elapsed = time.time() - start
        torch.cuda.empty_cache()
        print(f"{elapsed:.3f}s")
        return text.strip().lower()
    
    def contains_wakeword(self, text):
        "Once wake word detected, remove it and exe command follows"
        return any(wake_word in text for wake_word in self.wake_words)
    
    def extract_command(self, text):
        for wake_word in self.wake_words:
            if wake_word in text:
                parts = text.split(wake_word, 1)
                if len(parts) > 1:
                    command = parts[1].strip()
                    command = command.lstrip(",.?! ")
                    if command:
                        return command
        return ""
    
    def listen_loop(self):
        "Main listening loop"
        while True:
            # VAD detects speech
            audio = self.listen_with_vad()
            if audio is None:
                continue

            # Transcribe
            text = self.transcribe(audio)

            # Check for wake word
            if self.contains_wakeword(text):
                command = self.extract_command(text)
                print(f"Full text: {text}")
                if command:
                    print(f"Command: {command}")

                    if "my name is" in command.lower():
                        name_parts = command.lower().split("my name is")
                        if len(name_parts) > 1:
                            self.user_name = name_parts[1].strip().split()[0].capitalize()
                            print(f"Saved user name: {self.user_name}")

                    print("Thinking...")

                    system_prompt = "You are MERLIN, a helpful AI assistant"
                    if self.user_name:
                        system_prompt += f"You are speaking with {self.user_name}."
                    system_prompt += "Keep responses to 1-2 sentences."

                    response = self.llm.generate_response(
                        command,
                        memory_context = self.conversation_history,
                        system_prompt = system_prompt
                    )
                    print(f"Response: {response}")

                    self.conversation_history.append({
                        "user_input": command,
                        "merlin_response": response
                    })

                    if len(self.conversation_history) > 10:
                        self.conversation_history = self.conversation_history[-10:]

                    print("Speaking...")
                    self.tts.speak(response)
                else:
                    print("Command: (Wake word only no command detected)")
            else:
                print(f"No wake word: \"{text}\" (ignored)")

    
def main():
    brain = VoiceBrain()
    print("\n MERLIN voice brain test, phase1, Ctrl C to escape")
    try:
        brain.listen_loop()
    except KeyboardInterrupt:
        print("\n\n Shutting Down")
        del brain._whisper
        torch.cuda.empty_cache()
        gc.collect()
        print("See ya!")

if __name__ == "__main__":
    main()
