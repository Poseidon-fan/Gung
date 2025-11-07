# Gung

[![GitHub stars](https://img.shields.io/github/stars/Poseidon-fan/Gung)](https://github.com/rapiz1/Poseidon-fan/Gung)
[![GitHub release (latest SemVer)](https://img.shields.io/github/v/release/Poseidon-fan/Gung)](https://github.com/Poseidon-fan/Gung/releases)
![GitHub Workflow Status (branch)](https://img.shields.io/github/actions/workflow/status/Poseidon-fan/Gung/ci.yaml?branch=master)

> Gung is an intranet penetration tool, helping you expose a local server behind a NAT or firewall to the internet. The name has two meanings: firstly, it's an abbreviation of gungnir, the spear of Odin, the chief god in Norse mythology, symbolizing penetration and precision; secondly, it's a homophone for "Going Unblocked Network Gateway." — And the idea for this name originated from LLM.

## Features
- **Rich Protocol Integration** The transport and proxy layers are unified and abstracted. Multiplexed long-lived connections are encapsulated at the transport layer, currently supporting TCP and QUIC. The proxy layer currently supports TCP and HTTP, with HTTP using port multiplexing based on the host.
- **Pluggable** Currently supports user write Python codes as an embedded plugin to authenticate. Lua / RPC plugins will be supported later. Also, the plugin will support network traffic hijacking analysis, similar to nginx.

## Quicstart
Gung has two command-line tools: `gungs` for the server and `gungc` for the client, which can be obtained from the [release](https://github.com/Poseidon-fan/Gung/releases) page.

### Server
To start a server, run the following command on a machine with a public IP address:

```bash
$ gungs run <server_config.toml>
```

`<server_config.toml>` is the path to the configuration file; please refer to the [examples](examples/server_config/) for details. The minimal configuration items are as follows:
```toml
[transport]
# Transport type, tcp | quic
type = "tcp"
addr = "0.0.0.0:7777"

[auth]
# Allow all users to connect. Alternatively, it can be set to py_plugin to use a Python script for authentication.
type = "pass"

# Allow http proxy
[proxy.http]
base_domain = "your_server_domain"
```

### Client
`gungc` is a pure CLI tool, run `gungc --help` for details. A simple example is shown below

```bash
$ gungc <local_addr> \
    -s <server_addr> \
    -t tcp \
    -p http
```

## Planning

> This project is currently under active development. Feel free to star, fork, and submit PRs to help improve this project. 😊

- [ ] Support network traffic hijacking analysis. Integrate Lua and RPC plugin.
- [ ] More transport and proxy protocols.
- [ ] Http API and web ui for both server management and client analysis.
- [ ] Improve relevant documents
