# Configuration

The config file for Kawari is named `config.yaml`, using the [YAML format](https://en.wikipedia.org/wiki/YAML).

## Filesystem

Configuration for the game filesystem.

### `game_path`

Path to the game directory. For example, `C:\Program Files (x86)\SquareEnix\FINAL FANTASY XIV - A Realm Reborn\game`.

By default Kawari will attempt to find an existing game install for you, but this won't always work. For example, if you installed the game on a different hard drive.

```yaml
filesystem:
    game_path: C:\Program Files (x86)\SquareEnix\FINAL FANTASY XIV - A Realm Reborn\game
```

### `additional_search_paths`

Additional search paths for *unpacked game files*. These are ordered from highest-to-lowest, these are always preferred over retail game files.

### `unpack_path`

Unpack used files to the specified directory. If the directory is not specified, Kawari won't save file contents.

### `navimesh_path`

Navimesh file directory.

```yaml
filesystem:
    navimesh_path: naivmesh
```

### `additional_resource_paths`

Additional search paths for *resource files*. Needs to have the same folder structure as ours.

These are ordered from highest-to-lowest, and these are always preferred over our own resource files.

Note that drop-ins and timelines are *not* combined. Web templates do not respect this option.

## Admin

Configuration for the admin server.

### `port`

Port for the server.

```yaml
admin:
  port: 21057
```

### `listen_address`

Address to listen for new connections on.

```yaml
admin:
  listen_address: 0.0.0
```

### `server_name`

Public-facing domain of the server.

```yaml
admin:
  server_name: http://admin.ffxiv.localhost:21057
```

## Frontier

Configuration for the frontier server.

### `port`

Port for the server.

```yaml
frontier:
  port: 21058
```

### `listen_address`

Address to listen for new connections on.

```yaml
frontier:
  listen_address: 0.0.0.0
```

### `server_name`

Public-facing domain of the server.

```yaml
frontier:
  server_name: http://frontier.ffxiv.localhost:21058
```

### `worlds_open`

Controls whether launchers report that the world servers are open.

```yaml
frontier:
  worlds_open: true
```

### `login_open`

Controls whether launchers report that logins are open.

```yaml
frontier:
  login_open: true
```

## Lobby

Configuration for the lobby server.

### `port`

Port for the server.

```yaml
lobby:
  port: 21059
```

### `listen_address`

Address to listen for new connections on.

```yaml
lobby:
  listen_address: 0.0.0.0
```

### `server_name`

Public-facing domain of the server.

```yaml
lobby:
  server_name: 127.0.0.1
```

## Login

Configuration for the login server.

### `port`

Port for the server.

```yaml
login:
  port: 21060
```

### `listen_address`

Address to listen for new connections on.

```yaml
login:
  listen_address: 0.0.0.0
```

### `server_name`

Public-facing domain of the server.

```yaml
login:
  server_name: http://ffxiv-login.square.localhost:21061
```

### `enable_registration`

Whether registrations are enabled, and by default they are. Turning this off also hides the UI for this feature.

```yaml
login:
  enable_registration: true
```

## Patch

Configuration for the patch server.

### `port`

Port for the server.

```yaml
patch:
  port: 21061
```

### `listen_address`

Address to listen for new connections on.

```yaml
patch:
  listen_address: 0.0.0.0
```

### `patches_location`

Location of the patches directory on disk. Must be setup like so:

```ignore
<channel> (e.g. ffxivneo_release_game) /
    game/
    ex1/
...
```

```yaml
patch:
  patches_location: patches
```

### `server_name`

Public-facing domain of the server.

```yaml
patch:
  server_name: http://patch.ffxiv.localhost:21061
```

### `supported_platforms`

Determines which platform URLs are served.

```yaml
patch:
  supported_platforms:
  - win32
```

## Web

Configuration for the web server.

### `port`

Port for the server.

```yaml
web:
  port: 21062
```

### `listen_address`

Address to listen for new connections on.

```yaml
web:
  listen_address: 0.0.0.0
```

### `server_name`

Public-facing domain of the server.

```yaml
web:
  server_name: http://ffxiv.localhost:21062
```

## World

Configuration for the world server.

### `port`

Port for the server.

```yaml
world:
  port: 21063
```

### `healthcheck_port`

Port for the healthcheck endpoint.

```yaml
world:
  healthcheck_port: 21064
```

### `listen_address`

Address to listen for new connections on.

```yaml
world:
  listen_address: 0.0.0.0
```

### `server_name`

Public-facing domain of the server.

```yaml
world:
  server_name: 127.0.0.1
```

### `world_id`

What the World server is called. See the World Excel sheet.

```yaml
world:
  world_id: 63
```

### `enable_packet_obsfucation`

Enable packet obsfucation. There's literally no reason to do this!

```yaml
world:
  enable_packet_compression: false
```

### `enable_packet_compression`

Enable packet compression for packets from the server. It's recommended to keep this on.

```yaml
world:
  enable_packet_compression: true
```

### `login_message`

Default message received when logging into the world.

```yaml
world:
  login_message: Welcome to Kawari!
```

### `generate_navmesh`

Whether we generate new navmeshes on-demand. This consumes a lot of resources when entering a new zone!

```yaml
world:
  generate_navmesh: true
```

### `active_festivals`

The active festivals.

```yaml
world:
  active_festivals:
  - 0
  - 0 
  - 0
  - 0
  - 0
  - 0
  - 0
  - 0
```

### `accept_new_characters`

Whether the World should accept new characters.

```yaml
world:
  accept_new_characters: true
```

### `exp_bonus`

Whether the World has the EXP bonus.

```yaml
world:
  exp_bonus: false
```

### `language`

The language to read game data as, should have no effect on regular gameplay but definitely does affect a lot of debug/GM commands.

```yaml
world:
  language: en
```

## Tweaks

Configuration for various tweaks.

### `always_allow_skipping`

If true, allows the player to skip some cutscenes that are normally unskippable.

```yaml
tweaks:
  always_allow_skipping: false
```

### `enforce_validity_checks`

Enable various validity checks for version and file hashes that emulate retail.

```yaml
tweaks:
  enforce_validity_checks: true
```
