let element = document.getElementById('terminal');

/// Terminal Configuration
Terminal.applyAddon(fit);
window.term = new Terminal({ lineHeight: 1.25 });
window.term.open(document.getElementById('terminal'));
window.term.fit();

window.keyInputs = [];
window.pendingWrites = '';

window.term.on('key', function(char, _) {
    if (char === '\r') char = '\n';
    if (!window.vm.halted) {
        window.keyInputs.push(char.charCodeAt(0));
        window.requestAnimationFrame(window.schedule);
    }
});


/// Terminal IO
export function putChar(val) {
    let char = String.fromCharCode(val);
    if (char === '\n') char = '\r\n';

    if (window.pendingWrites.length === 0) {
        window.requestAnimationFrame(() => {
            window.term.write(window.pendingWrites);
            window.pendingWrites = [];
        });
    }
    window.pendingWrites += char;
}

export function hasChar() {
    return window.keyInputs.length > 0;
}

export function getChar() {
    return window.keyInputs.shift() || 0;
}

/// This is the code to handle dropping image files into the terminal.
element.addEventListener('dragover', (e) => e.preventDefault())
element.addEventListener('drop', (e) => {
    e.preventDefault();
    let file = e.dataTransfer.files[0];
    let reader = new FileReader();
    reader.onload = function(_) {
        let image = new Uint8Array(reader.result);
        window.vm.halted = true;
        if (window.vm.load_wasm(image)) {
            window.term.clear();
            window.keyInputs = [];
            window.pendingWrites = '';
        }
        window.requestAnimationFrame(window.schedule);
    }
    reader.onerror = function() {
        console.warn(reader.error);
    };
    reader.readAsArrayBuffer(file);
})