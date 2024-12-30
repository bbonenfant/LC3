# LC-3 VM

This is an LC-3 virtual machine following 
[this tutorial](https://www.jmeiners.com/lc3-vm/).
I adapted the tutorial to code this in Rust, and then I decided to see if I 
could compile it to WASM and get it running in the browser.

This will run locally in your terminal (UNIX only -- sorry Windows folks),
and can be run in the browser. The WASM version can be found hosted at
[bog.gy/lc3-vm](https://bog.gy/lc3-vm).

I'd like to come back to this toy at some point in the future try my hand
at writing some non-trivial assembly programs. Or maybe try to work through
the UX of building a debugger for this.

Example programs:
* [2048](https://www.jmeiners.com/lc3-vm/supplies/2048.obj) - 
  (written by rpendleton)
* [Rogue](https://www.jmeiners.com/lc3-vm/supplies/rogue.obj) -
  (written by jmeiners)

## Building
### Terminal
```shell
cargo run hello_world.obj
```

### Web/WASM
I built this using [wasm-pack](https://rustwasm.github.io/wasm-pack/)
```shell
wasm-pack build --target web
```
