# RLS
### Rust Logic Simulator

Logisim-inspired logic circuits simulator

### Global TODOs

- [Done] Place circuit boards as circuits
- [Done] Circuit controls 
- [Working] Circuit designer for circuit boards

- Rename the project

- UI descriptions, tooltips, hints, etc
- Better UI, more tabs
- Proper keybinds, some mobile support

- More components (LEDs, 7-segments...)
- Fully custom pins that can interface with other wires in other circuits (labels, seamless pins)

- Embed as `iframe`s
- Some sort of plugin api
- Separate into backend and frontend crates
- Figure out themes
- Proper mobile support
- Proper errors

## Local TODOs

- Rotate selection
- Remove wire with Wire tool
- Warning to replace circuits that don't match latest pin layout
- Make pins use random ID and store it in board data
- When copying pins, preserve their ID and design data
- Maybe show circuit pin labels in paste

![](progress_preview.png)

### Web version

Web version is available [here](https://ved-s.github.io/rls).
It's less precise due to browser limitations.