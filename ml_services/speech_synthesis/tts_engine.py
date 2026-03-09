"""TTS ENGINE"""
import subprocess
from pathlib import Path
import os

# suppress ONNX runtime warnings
os.environ['ORT_LOGGING_LEVEL'] = '3'

class TTSEngine:
    def __init__(self, voice_model = "en_US-lessac-medium"):
        """Init Piper TTS, ARGS: voice model name(in piper_voice env file)"""
        self.voice_model = voice_model
        self.model_path = Path.home() / ".local/share/piper-voices" / f"{voice_model}.onnx"

        if not self.model_path.exists():
            raise FileNotFoundError(
                f"Voice model not found: {self.model_path}\n"
            )
        print(f"TTS Engine: Ready {voice_model}")

    def speak(self, text: str, blocking = True):
        """Speak text, ARGS: text to speak, blocking or non-blocking"""
        if not text or not text.strip():
            return
        print(f"MERLIN: {text}")

        try:
            # TTS Pipeline: text -> piper -> raw audio -> aplay (output audio for ALSA)
            piper_cmd = [
                "piper",
                "--model", str(self.model_path),
                "--output-raw"
            ]

            aplay_cmd = [
                "aplay",
                "-r", "22050", # Sample rate
                "-t", "raw", # Raw audio
                "-f", "S16_LE", # Format: 16-bit signed little-endian
                "-q", # quiet mode
                "-" # read from stdin
            ]

            # Start piper process
            piper_process = subprocess.Popen(
                piper_cmd,
                stdin = subprocess.PIPE,
                stdout = subprocess.PIPE,
                stderr = subprocess.DEVNULL # suppress ONNX warning
            )

            # Start aplay process
            aplay_process = subprocess.Popen(
                aplay_cmd,
                stdin = piper_process.stdout,
                stdout = subprocess.DEVNULL,
                stderr = subprocess.DEVNULL
            )

            # Send text to piper
            piper_process.stdin.write(text.encode())
            piper_process.stdin.close()

            if blocking:
                # Wait for piper to finish
                aplay_process.wait()
                piper_process.wait()
        except Exception as e:
            print(f"TTS Error: {e}")


#Test
if __name__ == "__main__":
    print("Testing TTS Engine...")
    tts = TTSEngine()
    tts.speak("Hello! I am MERLIN, your AI assistant.")
    tts.speak("I can speak to you now!")
