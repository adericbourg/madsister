import { openUrl } from "@tauri-apps/plugin-opener";
import { useRef } from "react";

const AUTHOR = "Alban Dericbourg";
const REPO = "https://github.com/adericbourg/madsister";

const MODELS = [
  ["madmom", "CPJKU", "BSD code, weights CC BY-NC-SA 4.0", "https://github.com/CPJKU/madmom"],
  ["all-in-one", "mir-aidj", "MIT", "https://github.com/mir-aidj/all-in-one"],
  ["Demucs", "Meta / Alexandre Défossez", "MIT", "https://github.com/adefossez/demucs"],
  ["BTC", "jayg996", "MIT", "https://github.com/jayg996/BTC-ISMIR19"],
  ["Chord-CNN-LSTM", "music-x-lab", "MIT", "https://github.com/music-x-lab/ISMIR2019-Large-Vocabulary-Chord-Recognition"],
] as const;

/** All under the SIL Open Font License 1.1. */
const FONTS = [
  ["Patrick Hand", "Patrick Wagesreiter", "https://github.com/google/fonts/tree/main/ofl/patrickhand"],
  ["Kalam", "Indian Type Foundry", "https://github.com/google/fonts/tree/main/ofl/kalam"],
  ["Petaluma", "Steinberg Media Technologies", "https://github.com/steinbergmedia/petaluma"],
] as const;

export function About() {
  const dialog = useRef<HTMLDialogElement>(null);
  const close = () => dialog.current?.close();
  const link = (name: string, url: string) => (
    <a
      href={url}
      onClick={(e) => {
        e.preventDefault();
        void openUrl(url);
      }}
    >
      {name}
    </a>
  );

  return (
    <>
      <button type="button" onClick={() => dialog.current?.showModal()}>
        About
      </button>
      <dialog
        ref={dialog}
        className="about"
        aria-labelledby="about-title"
        onClick={(e) => e.target === e.currentTarget && close()}
      >
        <div>
        <h2 id="about-title">About madsister</h2>
        <h3>Author</h3>
        <p>{AUTHOR}</p>
        <h3>Models</h3>
        <ul>
          {MODELS.map(([name, author, license, url]) => (
            <li key={name}>
              {link(name, url)} by {author}, {license}
            </li>
          ))}
        </ul>
        <h3>Fonts</h3>
        <ul>
          {FONTS.map(([name, author, url]) => (
            <li key={name}>
              {link(name, url)} by {author}, SIL Open Font License 1.1
            </li>
          ))}
        </ul>
        <p>{link("Source code on GitHub", REPO)}</p>
        <button type="button" autoFocus onClick={close}>
          Close
        </button>
        </div>
      </dialog>
    </>
  );
}
