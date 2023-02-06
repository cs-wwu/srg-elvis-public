# <a id="top"></a> NDL Networks Specifications

Core network specifications are as follows.

- In order to start a networks section you must define it with no tabs with [`[Networks]`](#networks-type)
- Inside of the `[Networks]` sections (now 1 tab in), you need to define a [`[Network]`](#network-type) with an ID.
  - Example: `[Network id='5']`
- Each Network section can have subsections of [IPs or IP ranges](#ip-type). This will now be 2 tabs in.
  - Example of IP range: `[IP range='123.45.67.89/91']`
  - Example of single IP: `[IP ip='192.168.1.121']`

# <a id="networks-type"></a> Networks Type

## Parameters

`[Networks]` will take in **no parameters**, and simply just defines the start of a new networks section. To  define a network, see [here](#network-type).

# <a id="network-type"></a> Network Type

A Network type mostly acts as a definition, but gets assigned an id where it can be referenced elsewhere in the sim.

## Parameters

`[Network]` **must** take in the following as a parameter:

- `id` **(Required)**
  - This can be defined as `[Network id='1']`

# <a id="ip-type"></a> IP Type

There are two IP types, IP range and single ip. All IP definintions use standard CIDR notation. You can list as many IPs or IP ranges in each network, however no overlapping IP's are allowed in the same network, or across networks.

## Parameters

`[IP]` **must** take in at least one of the following as parameters:

- `ip` **(One required)**
  - This can be defined as `[IP ip='192.168.1.121']`
- `range` **(One required)**
  - This can be defined as `[IP range='123.45.67.89/91']`
