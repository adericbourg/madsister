const FONTS = [
  ["Patrick Hand", "Patrick Wagesreiter", "https://github.com/google/fonts/tree/main/ofl/patrickhand"],
  ["Kalam", "Indian Type Foundry", "https://github.com/google/fonts/tree/main/ofl/kalam"],
  ["Petaluma", "Steinberg Media Technologies", "https://github.com/steinbergmedia/petaluma"],
  ["Golden Age", "Don Rice, curated by Ben Byram-Wigfield", "https://github.com/benwiggy/GoldenAge"],
] as const;

/** Credits for the chart fonts, all under the SIL Open Font License 1.1. */
export function FontCredits() {
  return (
    <details>
      <summary>About</summary>
      <ul>
        {FONTS.map(([name, author, url]) => (
          <li key={name}>
            <a href={url} target="_blank" rel="noreferrer">
              {name}
            </a>{" "}
            by {author}, SIL Open Font License 1.1
          </li>
        ))}
      </ul>
    </details>
  );
}
