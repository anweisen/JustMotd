# JustMotd
 A configurable socket server written in rust responding to packets via the minecraft protocol, providing a motd and disconnect information - for discontinued server projects

## Setup

## Configuration

### Config File

```json5
{
  // the local address to bind the listener to
  "bind": "0.0.0.0:25565", 

  // the path to the favicon file as png, must be exactly 64x64! (ignored if not existent)
  "favicon": "path.png",
  
  "motd": {
	// the motd text for pre1.16 clients, line separation with \n, colors with §
	"text": "pre 1.16 text",
	
	// 1.16 & older support custom rgb colors, fallback to "text" above if not set
	"component": {
	  // generated with tools like https://colorize.fun/en/minecraft & https://minecraft.tools/en/json_text.php
	}
  },
  
  "version": {
	// the version name, colors with §
	"text": "version text instead of player count",
	
	// text shown when hovering over version text, colors with §
	"hover": "hover text through sample players"
  },
  
  "disconnect": {
    // the disconnect text for pre1.16 clients, line separation with \n, colors with §
    "text": "pre 1.16 text",

    // 1.16 & older support custom rgb colors, fallback to "text" above if not set
    "component": {
      // generated with tools like https://colorize.fun/en/minecraft & https://minecraft.tools/en/json_text.php
    }
  }
}
```
