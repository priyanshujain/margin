import { Library } from "./components/Library";
import { EditorView } from "./components/EditorView";
import { useBook } from "./store/useBook";

function App() {
  const book = useBook((s) => s.book);
  const openBook = useBook((s) => s.openBook);

  return book ? <EditorView /> : <Library onOpen={openBook} />;
}

export default App;
