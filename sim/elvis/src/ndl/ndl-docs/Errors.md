# NDL Basic Error Understanding

# Parsing Errors

Parsing errors may be returned during the parsing stage of the NDL, if you have failed some of the formatting guidelines. Errors such as missing tabs, spaces, or brackets are quite common.
Typical parsing error messages look as follows
```
Line 9: Unable to parse inside of Machines due to: 
        Line 10: Unable to parse inside of Machine due to: 
            Line 16: Unable to parse inside of Applications due to: 
                Line 16: expected type Application and got type Network instead.
```
Note that it will print the line number it occured on followed by the reason why it errored. The errors follow a stack down, meaning the top most line will be the highest level of error, in this case inside of the Machines section. Additionally the tabbing on the error will match the tabbing of your NDL document, as to match to your code easier. The bottom most line will most likely be the core error the parser encountered.

# Generator Errors

Generator errors will look far different than Parsing errors. This is due to the fact that we no longer have line number to associate too and will instead reference variables themselves. These errors will make it through the parser but have a core problem will the construction of them, common ones are invalid application names and bad IP's or ports. These errors will usually come from either an assert, or an thread panic due to the fact that it is constructing the sim at this stage.

A thread panic looks as follows
```
thread 'main' panicked at 'Invalid application in machine. Got application bad_application', elvis\src\ndl\generating\machine_generator.rs:233:25
```
Note that this contains the error itself, the value that was incorrect, and the line number of the generator it errored on.