import { useState } from "react";
import { emptySong } from "./model/song";
import { Grid } from "./ui/Grid";

function App() {
  const [song] = useState(emptySong);
  return (
    <main>
      <Grid song={song} barsPerRow={4} style="fr" cursor={null} />
    </main>
  );
}

export default App;
