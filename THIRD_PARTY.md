# Third-party components

madsister's own code is GPLv3 (see `LICENSE`). Model weights are never committed: they are downloaded at setup.
Several weights are CC BY-NC-SA, so the whole application is for **non-commercial use only**.

| Component | Purpose | Code license | Weights license | Source |
|---|---|---|---|---|
| madmom (CPJKU) | Beats, downbeats (`beats-madmom` group) | BSD | CC BY-NC-SA 4.0 (shipped inside the package) | https://github.com/CPJKU/madmom (git `main`, pinned sha) |
| all-in-one / `allin1` (mir-aidj) | Beats, downbeats, sections (`beats-allinone` group) | MIT | MIT (Hugging Face model card `taejunkim/allinone`) | https://github.com/mir-aidj/all-in-one, https://huggingface.co/taejunkim/allinone |
| NATTEN (SHI-Labs) | Neighborhood attention ops used by all-in-one, built from source on macOS | MIT | — | https://github.com/SHI-Labs/NATTEN (0.17.4) |
| Demucs / HTDemucs (Meta, adefossez) | Source separation: inside all-in-one, and the harmonic stem (`separate` group) | MIT | MIT per the repo license (the `adefossez/HTDemucs` model card states none) | https://github.com/adefossez/demucs, https://huggingface.co/adefossez/HTDemucs |
| BTC (jayg996, ISMIR'19) | Large-vocabulary chords (`chords-btc` group), repo cloned at setup | MIT | MIT (`test/btc_model_large_voca.pt` ships in the MIT-licensed repo) | https://github.com/jayg996/BTC-ISMIR19 (pinned sha `2682317`) |
| Chord-CNN-LSTM (music-x-lab, ISMIR'19) | Large-vocabulary chords with inversions (`chords-cnnlstm` group), repo cloned at setup | MIT | MIT (`cache_data/*.sdict` ship in the MIT-licensed repo) | https://github.com/music-x-lab/ISMIR2019-Large-Vocabulary-Chord-Recognition (pinned sha `481f4ce`) |
| h5py, joblib, pretty_midi, pydub | Imported by Chord-CNN-LSTM's `mir` package (not used for inference) | BSD / BSD / MIT / MIT | — | PyPI |
| mir_eval | Imported by BTC's feature code; chord/beat metrics | MIT | — | https://github.com/mir-evaluation/mir_eval |
