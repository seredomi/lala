# lala

## goal

create a tool to separate piano from audio files. specifically some [moondog](https://en.wikipedia.org/wiki/Moondog) songs I really love. maybe from there I can convert them into sheet music. we'll see

## stack

**frontend**:

- [tauri](https://v2.tauri.app/)
- [typescript](https://www.typescriptlang.org/)
- [react](https://react.dev/)
- [carbon](https://carbondesignsystem.com/)

**backend**:

- [rust](https://www.rust-lang.org/)
- [tch-rs](https://github.com/LaurentMazare/tch-rs)
- [demucs](https://github.com/facebookresearch/demucs)

## progress

- [x] upload song
- [x] separate tracks
- [ ] handle multiple files, store results
- [ ] transcribe piano stem to midi
- [ ] convert midi to sheet music pdf
