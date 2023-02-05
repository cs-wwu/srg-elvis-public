# NDL Examples

The following will be some examples of full simulations.

To learn more about specifics, it is recommended to look at:

- [Machines](Machines.md)
- [Networks](Networks.md)

<br>

# Basic Message Sending

This simulation will create 1000 machines that each send one message to a receiving server.

```
[Networks]
	[Network id='5']
		[IP range='123.45.67.89/91']
		[IP range='123.45.67.92/94']
		[IP ip='192.168.1.121']
	[Network id='1']
		[IP range='12.34.56.89/90']
[Machines]
	[Machine name='test1' count='1000']
		[Networks]
			[Network id='5']
			[Network id='1']
		[Protocols]
			[Protocol name='IPv4']
			[Protocol name='UDP']
		[Applications]
			[Application name='send_message' message='Hello this is an awesome test message!' to='recv1' port='0xbeef']
	[Machine name='recv1']
		[Networks]
			[Network id='5']
		[Protocols]
			[Protocol name='IPv4']
			[Protocol name='UDP']
		[Applications]
			[Application name='capture' ip='123.45.67.90' port='0xbeef' message_count='1000']
```