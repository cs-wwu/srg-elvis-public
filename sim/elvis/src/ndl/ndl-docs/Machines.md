# <a id="machines-top"></a> NDL Machine Specifications

Core machine specifications are as follows.

- In order to start a machines section you must define it with no tabs with [`[Machines]`](#machines-type)
- Inside of this section you can define any number of machines each starting 1 tab in. Using the [`[Machine]`](#machine-type) tag
  - Example: `[Machine name='test1']`
- Each machine **must** then have the 3 following parameters. Those parameters must be defined 2 tabs in.
  - [`[Networks]`](#machine-networks-type)
  - [`[Protocols]`](#protocols-type)
  - [`[Applications]`](#applications-type)

# <a id="machines-type"></a> Machines Type

## Parameters

`[Machines]` will take in **no parameters**, and simply just defines the start of a new networks section. To define a machine, see [here](#machine-type).

# <a id="machine-type"></a> Machine Type

## Parameters

`[Machine]` can take in the following parameters:

- `name` **(Required)**
    - This can be defined as `[Machine name='test']`
- `count` **(Optional)**
    - This can be defined as `[Machine name='test' count='100']`

# <a id="machine-networks-type"></a> Networks Type

Not to be confused with [sim-level \[Networks\]](Networks), machine [Networks] exist to assign sim-level networks to a machine.

## Parameters

`[Networks]` will take in **no parameters**, and simply just defines the start of a new networks section. To define a network, see [here](#machine-network-type).

# <a id="protocols-type"></a> Protocols Type

## Parameters

`[Protocols]` will take in **no parameters**, and simply just defines the start of a new protocols section. To define a protocol, see [here](#protocol-type).

# <a id="applications-type"></a> Application Type

## Parameters

`[Applications]` will take in **no parameters**, and simply just defines the start of a new applications section. To define an application, see [here](#application-type).

# <a id="machine-network-type"></a> Network Type

## Parameters

A machine network **must** take in 1 `id` parameter. You may define as many network blocks as you would like for the machine to connect to, but never duplicate networks. The network must exist in a [sim-level \[Networks\] definition](Networks).

- `id` **(Required)**
  - This can be defined as `[Network id='1']`

# <a id="protocol-type"></a> Protocol Type

## Parameters

Machine protocol **must** take in a `name` parameter. You may define as many protcol blocks as you would like for the machine, but never duplicate protocols. See [here](Protocol) for protocol definitions.

- `name` **(Required)**
  - This can be defined as `[Protocol name='IPv4']`

# <a id="application-type"></a> Application Type

## Parameters

Machine application **must** take in at minimum a `name` parameter. Each application will have its own input types and may require more parameters depending on the application. You may define as many application blocks as you would like for the machine, but never duplicate applications. See [here](Application) for application definitions.

- `name` **(Required)**
  - This can be defined as `[Application name='send_message']`
  - send_message requires more definitions. Those parameters may look like the following.
    - `[Application name='send_message' message='Hello this is an awesome test message!' to='recv1' port='0xbeef']`

## [Back to Top](#machines-top)