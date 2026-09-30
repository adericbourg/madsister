import { useState } from "react";
import { emptySong } from "./model/song";
import { Editor } from "./ui/Editor";
import { useHistory } from "./ui/useHistory";

function App() {
  const [initial] = useState(emptySong);
  const history = useHistory(initial);
  return (
    <main>
      <Editor history={history} barsPerRow={4} style="fr" />
    </main>
  );
}

export default App;
