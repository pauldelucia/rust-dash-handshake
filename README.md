# rust-dash-handshake
Handshake with a Dash full node

This program connects to a Dash full node given its IP address, sends a version message, awaits a version message in response, and then sends a verack message. It logs a successful handshake when it receives a version message and either a verack or inv message from the node.

IP addresses of Dash masternodes can be found with various resources such as mnowatch.org. A list of Binance-hosted masternode IP addresses can be found here https://mnowatch.org/binance/.

There are two places where the IP address needs to be input into the program - one in vector form and one in string form.
