<p align="center">
  <img src="https://upload.wikimedia.org/wikipedia/de/9/96/FrozenThroneLogo.gif?raw=true" width="300" /> 
</p>

# Warcraft III Frozen Throne Proxy 
A simple tool that let you play warcraft III frozen throne over vpn.

## Description 
This tool is a tcp proxy combined with a udp discover mechanism. This enables you to play Warcraft III Frozen Throne over more complex network configurations like a VPN (Zerotier One, Hamachi). This tool is inspired by the https://gaming-tools.com/warcraft-3/lancraft/ tool.

![alt text](https://raw.githubusercontent.com/exellian/wc3-ft-proxy/main/assets/example.png?raw=true)



## Tutorial
1. Ensure that all players are connected over LAN (Zerotier One, Hamachi)
2. Ensure that all players have set the same game port: options -> gameplay -> game port (e.g: 6112)
3. Ensure that the host has disabled the windows defender firewall or has set the correct firewall rules for the used game port
4. Ensure that the host doesn't run the proxy tool (could work but I didn't tried it yet)
5. All **other** players that want to join have to run the proxy tool
6. Enter the ip address and port of the **host** (ipv6 will also probably work)
-> Now all players that want to join should find the server!

## Support
Only Windows
