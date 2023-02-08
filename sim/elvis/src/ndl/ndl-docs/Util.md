# NDL Utility Helpers Specification

These methods are here to help with the implementation of applications to the generator. Here are the currently available utility helper functions:

- [String To Port](#string_to_port)
- [IP String to IP](#ip_string_to_ip)
- [IP or Name](#ip_or_name)

# Adding New Utility Helpers

In the `generator_utils` file in the generator folder of the NDL you will find the current implementations of the base helper functions. In this file you can add new utilities and have full access to them via auto import into the Application Generator and Machine generator. 

# <a id="string_to_port"></a> String To Port

This method will convert from either a hex value or decimal value in String form and turns it into a u16 as a port. This input should be taken from the arguments provided to an application, specifically a port argument. Will error upon an invalid port.

## Extra Arguments

This protocol takes in the following arguments:
- `p`
    - **Definition**: A hex value or decimal value
    - **Type**: String

# <a id="ip_string_to_ip"></a> IP String to IP

This method will convert an IP String into a [u8; 4] array, which is the core IP type. Will error on invalid octets in IP or invalid formatting.

## Extra Arguments

This protocol takes in the following arguments:
- `s`
    - **Definition**: An IP
    - **Type**: String
    - **Example** `"192.168.1.121" into [192, 168, 1, 121]`

# <a id="ip_or_name"></a> IP or Name

This method will detect if a given String is either an IP or a name for a machine. Returns `True` for IP and `False` for name. Assumes any non-valid IP is a valid name and will not error upon invalid names.

## Extra Arguments

This protocol takes in the following arguments:
- `s`
    - **Definition**: Either an IP or Machine name
    - **Type**: String