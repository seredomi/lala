# uv run --with torch --with numpy --with demucs --with torchaudio src-tauri/src/script.py

import torch
from torchaudio.pipelines import HDEMUCS_HIGH_MUSDB_PLUS

bundle = HDEMUCS_HIGH_MUSDB_PLUS
model = bundle.get_model()
model.eval()

# Example input: [batch, channels, samples]
dummy_input = torch.randn(1, 2, 441000)  # 10 seconds stereo

# Export to TorchScript
traced = torch.jit.trace(model, dummy_input)
traced.save("hdemucs.pt")
