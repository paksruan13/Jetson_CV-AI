#!/usr/bin/env python3

import os
os.environ["PYTORCH_CUDA_ALLOC_CONF"] = 'backend:native, expandable_segments:True'
os.environ["PYTORCH_NO_CUDA_MEMORY_CACHING"] = '1'

import gc
import torch

#Clear GPU before init
torch.cuda.empty_cache()
gc.collect()

from faster_whisper import WhisperModel
import sounddevice as sd
import numpy as np
import scipy.signal
import time

print("Faster-Whisper GPU Test (Mem management added)")

#Mic config
device_info = sd.query_devices(0, 'input')
mic_rate = int(device_info['default_samplerate'])
print(f"\nMicrophone: {device_info['name']} at {mic_rate} Hz")

#Load Model
print(f"\nLoading Whisper (FP16)...")
try:
    model = WhisperModel("tiny", device="cuda", compute_type="float16")
    load_time = time.time()
    print(f"Load Successful")

    #Check mems
    allocated = torch.cuda.memory_allocated(0) / 1024**2
    print(f"Model Size: {allocated:.1f} MB")

except RuntimeError as e:
    print(f"Failed: {e}")
    exit(1)

#Record
print(f"\n Recording 5 seconds...")
time.sleep(1.5)

audio = sd.rec(
    int(5 * mic_rate),
    samplerate = mic_rate,
    channels = 1,
    dtype = np.float32,
    device = 0
)
sd.wait()

# Resample to 16000 Hz
if mic_rate != 16000:
    num_samples = int(len(audio) * 16000 / mic_rate)
    audio = scipy.signal.resample(audio, num_samples).astype(np.float32)

# Transcribe
print(f"\nTranscribing...")
torch.cuda.synchronize()
start = time.time()

segments, info = model.transcribe(audio.flatten(), language = "en")
result = ' '.join([s.text for s in segments])

torch.cuda.synchronize()
trans_time = time.time() - start

#Print res
print("Results:")
print(f"{result.strip()}")
print(f"\n Transcription Time: {trans_time:.3f} seconds")
print(f"Status: {'FAST' if trans_time < 0.5 else 'OK'}")

#cleanup
del model
torch.cuda.empty_cache()
gc.collect()

print("GPU mems cleaned up!")
