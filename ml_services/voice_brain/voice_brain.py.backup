"""MERLIN Core Pipeline: Brain"""

import os
os.environ["PYTORCH_CUDA_ALLOC_CONF"] = 'backend:native, expandable_segments:True'

import torch
import sounddevice as sd
import gc
import numpy as np
import scipy.signal
import time
from faster_whisper import WhisperModel
import webrtcvad

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

        self.wake_words = ["merlin", "hey merlin"]

        #Audio Setting
        self.sample_rate = 16000
        self.chunk_duration = 0.02
        self.frame_samples = int(self.sample_rate * self.chunk_duration)
        self.silence_duration = 2.0 #2s silence to indicate end of speech

        print(f"Wake words : {self.wake_words}")
    
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
            #check if speech
            if self.is_speech(chunk_16k):
                if not speech_started:
                    if not hasattr(callback, "speech_frames"):
                        callback.speech_frames = 0
                    callback.speech_frames += 1
                    if callback.speech_frames >= 3:
                        print("\n Speech deteced!", end="", flush=True)
                        speech_started = True
                        callback.speech_frames = 0
                else:
                    chunks.append(chunk_16k)
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
                    command = command.lstrip(",.?!")
                    return command
        return text
    
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
                print(f"Command: {command}")
            
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