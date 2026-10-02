# Third-party components

madsister's own code is GPLv3 (see `LICENSE`). Model weights are never committed: they are downloaded at setup.
Several weights are CC BY-NC-SA, so the whole application is for **non-commercial use only**.

| Component | Purpose | Code license | Weights license | Source |
|---|---|---|---|---|
| madmom (CPJKU) | Beats, downbeats (`beats-madmom` group) | BSD | CC BY-NC-SA 4.0 (shipped inside the package; the Linux release packages bundle a madmom wheel built in CI, weights included) | https://github.com/CPJKU/madmom (git `main`, pinned sha) |
| all-in-one / `allin1` (mir-aidj) | Beats, downbeats, sections (`beats-allinone` group) | MIT | MIT (Hugging Face model card `taejunkim/allinone`) | https://github.com/mir-aidj/all-in-one, https://huggingface.co/taejunkim/allinone |
| NATTEN (SHI-Labs) | Neighborhood attention ops used by all-in-one, built from source on macOS | MIT | — | https://github.com/SHI-Labs/NATTEN (0.17.4) |
| Demucs / HTDemucs (Meta, adefossez) | Source separation: inside all-in-one, and the harmonic stem (`separate` group) | MIT | MIT per the repo license (the `adefossez/HTDemucs` model card states none) | https://github.com/adefossez/demucs, https://huggingface.co/adefossez/HTDemucs |
| BTC (jayg996, ISMIR'19) | Large-vocabulary chords (`chords-btc` group), repo downloaded at setup | MIT | MIT (`test/btc_model_large_voca.pt` ships in the MIT-licensed repo) | https://github.com/jayg996/BTC-ISMIR19 (pinned sha `2682317`) |
| Chord-CNN-LSTM (music-x-lab, ISMIR'19) | Large-vocabulary chords with inversions (`chords-cnnlstm` group), repo downloaded at setup | MIT | MIT (`cache_data/*.sdict` ship in the MIT-licensed repo) | https://github.com/music-x-lab/ISMIR2019-Large-Vocabulary-Chord-Recognition (pinned sha `481f4ce`) |
| h5py, joblib, pretty_midi, pydub | Imported by Chord-CNN-LSTM's `mir` package (not used for inference) | BSD / BSD / MIT / MIT | — | PyPI |
| mir_eval | Imported by BTC's feature code; chord/beat metrics | MIT | — | https://github.com/mir-evaluation/mir_eval |
| PyTorch, torchaudio | Inference for every model group (pinned 2.5.1, see `doc/roadmap/BLOCKERS.md`) | BSD-3-Clause | — | https://github.com/pytorch/pytorch, https://github.com/pytorch/audio |
| NumPy | Core dependency | BSD-3-Clause | — | https://numpy.org |
| librosa | Audio features for BTC and Chord-CNN-LSTM, chroma for the add2/add4 heuristic | ISC | — | https://github.com/librosa/librosa |
| PyYAML | BTC's config loading | MIT | — | https://github.com/yaml/pyyaml |
| yt-dlp | Downloads a URL's audio (`fetch` group), the only network access after setup | Unlicense | — | https://github.com/yt-dlp/yt-dlp |
| python-sounddevice | Microphone recording (`record` group) | MIT | — | https://github.com/spatialaudio/python-sounddevice |
| PortAudio | Audio I/O under sounddevice (bundled in its macOS wheels; `libportaudio2` on Linux) | MIT | — | https://www.portaudio.com |
| FFmpeg | Decoding (called as an external program on `PATH`, not bundled) | LGPL-2.1+ (GPL-2+ for some builds) | — | https://ffmpeg.org |
| uv (Astral) | Bundled in the release packages (`madsister-uv`, `madsister-snapshot-uv`): installs Python and the engine on first launch | MIT OR Apache-2.0 | — | https://github.com/astral-sh/uv |
| GStreamer (base, good, libav plugins), WebKitGTK and their libraries | Bundled in the Linux AppImage only (the deb depends on the distribution's packages) | LGPL-2.1+ (libav plugin: LGPL, FFmpeg inside) | — | https://gstreamer.freedesktop.org, https://webkitgtk.org |
| pytest | Tests only (`dev` group), not shipped | MIT | — | https://github.com/pytest-dev/pytest |
| GuitarSet (Xi et al., ISMIR 2018) | Proxy bench data (`bench/`), downloaded by `bench/fetch_guitarset.py`; not shipped | — | CC BY 4.0 (data) | https://zenodo.org/records/3371780 |
