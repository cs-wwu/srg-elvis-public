# NDL Documentation

This documentation is for the Network Description Language (NDL) for the ELVIS project. Using this language, it is possible to create large scale simulations that would otherwise not be possible. The goal of this language is to allow the creation of these simulations in the simplest and most readable format possible.

## Basic Formatting

This language was built using an indentation model, similar to Python. The idea is that using tabs, size set to 4, we can create an easily understood heirarchy of definitions for the NDL. Basic formatting goes as follows.

```
[Header Section]
	[Sub-Heading]
		[Specifications]
```
This example show that for each layer "down" you go in the language, you add one more tab. This allows for sections such as [\[Networks\]](Networks) to be completed sepcified seperately from other sections such as [\[Machines\]](Machines). This allows allows for the sections to be easily understood as anything tabbed in further than the line above is a part of that section.

## Parameters

Parameters are additonal arguments that can be added to appropriate definitions. Formatting follows these guidelines: name of the parameter followed by and equal sign, then the `'` character, the data for the parameter (such as an IP or machine name), then finally an ending `'`. Note that there are no spaces in this sequence, it must be `name='data'`.

### Examples

- `[Network id='5']`
  - This will have an additonal argument of `id` which will be set to the number `5`
- `[Application name='send_message' message='Hello this is an awesome test message!' to='recv1' port='0xbeef']`
  - Note in this circumstance name is a required field to application definitions, the rest are required by the application itself
  - The arguments pulled from this are:
    - `name` set to `send_message`
    - `message` set to `Hello this is an awesome test message!`
    - `to` set to `recv1`
    - `port` set to `0xbeef`

## Usage

In order to run the simulation, the command that needs to be run is the following:

```cargo run -- --ndl filepath```

"filepath" in this case can be either an absolute path (starting with a /) or a relative path from the NDL folder inside of the simulation.

The simulation can also be run in release mode, or also with logging enabled. Both are demonstrated in the following snippet:

```cargo run --release -- --ndl filepath --log```

## Links

[Networks](Networks) <br>
[Machines](Machines) <br>
[Applications](Application) <br>
[Protocols](Protocol) <br>
[Examples](Examples) <br>
[Errors](Errors) <br>
