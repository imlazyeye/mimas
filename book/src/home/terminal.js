// the home page's terminal. the repl runs in a worker so a `loop {}` can't freeze the tab, and reset
// throws the worker away and puts the terminal back the way the page loaded it
const terminal = document.getElementById('repl');
const screen = terminal.querySelector('.terminal-screen');
const output = terminal.querySelector('.terminal-output');
const input = terminal.querySelector('textarea');
const placeholder = input.placeholder;
let worker = null;

terminal.querySelector('button').addEventListener('click', () => {
  worker?.terminate();
  worker = null;
  output.replaceChildren();
  input.value = '';
  input.readOnly = false;
  input.placeholder = placeholder;
  document.activeElement.blur();
});
input.addEventListener('keydown', (event) => {
  // mdBook takes `?` and the arrow keys for its own shortcuts otherwise
  event.stopPropagation();
  if (event.key !== 'Enter' || event.shiftKey || event.isComposing) return;
  event.preventDefault();
  // read-only while an input runs, since it's echoed and cleared once it's done
  if (input.readOnly) return;
  // the wasm only downloads once someone runs something
  if (!worker) {
    worker = new Worker(new URL('./worker.js', import.meta.url), { type: 'module' });
    worker.onmessage = ({ data }) => {
      input.readOnly = false;
      // an unfinished input (an open `{`) stays put and gets another line
      if (data.unfinished) {
        input.value += '\n';
        return;
      }
      print('input', `> ${input.value}`);
      print('printed', data.printed);
      print('echo', data.echo);
      print('error', data.error);
      input.value = '';
    };
  }
  input.readOnly = true;
  input.placeholder = '';
  worker.postMessage(input.value);
});

function print(kind, text) {
  if (!text) return;
  const div = document.createElement('div');
  div.className = kind;
  div.textContent = text;
  output.append(div);
  screen.scrollTop = screen.scrollHeight;
}
