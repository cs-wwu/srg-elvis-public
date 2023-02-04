# NDL Documentation

This documentation is for the Network Description Language (NDL) for the ELVIS project. Using this language, it is possible to create large scale simulations that would otherwise not be possible. The goal of this language is to allow the creation of these simulations in the simplest and most readable format possible.

## Basic Formatting

This language was built using an indentation model, similar to Python. The idea is that using tabs, size set to 4, we can create an easily understood heirarchy of definitions for the NDL. Basic formatting goes as follows.

```
[Header Section]
	[Sub-Heading]
		[Specifications]
```
This example show that for each layer "down" you go in the language, you add one more tab. This allows for sections such as [\[Networks\]](Networks.md) to be completed sepcified seperately from other sections such as [\[Machines\]](Machines.md). This allows allows for the sections to be easily understood as anything tabbed in further than the line above is a part of that section.

TODO parameter formatting

## Links

[Networks](Networks.md) <br>
[Machines](Machines.md) <br>
[Applications](Application.md) <br>
[Protocols](Protocol.md) <br>
[Examples](Examples.md) <br>
[Errors](Errors.md) <br>
