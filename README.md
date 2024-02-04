# JustMotd

A configurable socket server written in rust responding to packets via the minecraft protocol, 
providing a motd and disconnect information - for discontinued server projects or maintenance

## Setup

### Docker Compose

```yaml
version: "3"

services:
  server:
    image: anweisen/just_motd:latest
    ports:
      - "25565:25565"
    volumes:
      - "./volume:/app/conf"
    environment:
      - CONFIG=conf/config.json
```


#### Start

```shell
docker compose up
```

#### Stop

```shell
docker compose down
```

### Docker Run

```shell
docker run -d -p 25565:25565 --name just_motd anweisen/just_motd:latest
```

#### With config

```shell
docker run -d -p 25565:25565 -v /your/path/to/conf:/app/conf -e CONFIG=conf/config.json --name just_motd anweisen/just_motd:latest
```

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
	
    // the motd text for legacy clients (1.6 and older), only one short line, no colors in 1.3 and older
    "legacy": "pre 1.16 text",
	
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
