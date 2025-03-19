# Basic Landlord Cache Simulator (csim)
Hi! Welcome to my cache simulator. I built this for my thesis in Spring 2025 off of an existing port
that I had made in C++ in the summer of 2024. I also expanded the functionality a little bit so that this version could read traces off of formatted TOML files instead of having to enter the trace into the command line each time.
## How to use this
Using this software is pretty simple: simply compile it with `cargo build`, find the executable,
and run that by specifying the relative path to a TOML file that contains your trace. Alternatively,
you may enter the `--no-toml` flag to enter all of these values manually. This will open an
interactive prompt where you can enter the refresh scalar of your Landlord implementation (0 for
FIFO-Landlord and 1 for LRU-Landlord), your cache size and your desired tie-breaking algorithm. At
the moment, this file only contains two tie-breaking algorithms (LRU and FIFO) but you're free to
add more! You can also enter these values directly by adding a space between your relative path to
the TOML file and then entering them into the command line. Just enter the string corresponding to
the tiebreaking algorithm (my parsing for the tiebreaking algorithm is case-agnostic but yours might
not be).
## Formatting of the TOML file
In order for the executable to properly understand your TOML file, it needs to be formatted in a
specific way so that `serde` can properly de-serialize it into a `trace` data structure. `serde`
expects a few things:
1. A table called `items`. This will contain information about each item. For instance, if you have
   an item called `A0`, then you should have fields `A0.cost` and `A0.size` with integer values 
   afterward.These integer values can be anything but you need to have both fields initialized or
   else the program will produce an error. **Make sure that each item has a unique identifying
   string!** This is to avoid ambiguity.
2. A table called `trace`. This will contain a numbered list of the items that you would like to
   request from the cache in this trace. **Each of them items that are requested should be specified
   in your items!** The executable will throw an error if it finds a request to an item not
   specified in your items table.
And that's it! `serde` will handle de-serializing this into a trace and the executable will run it
with your specified Landlord variant. An example TOML file is provided with `items.toml`.
*NOTE:* I realize that it might be kind of annoying to add items into the middle of a numbered list
in this way if you use a text editor like VS Code. To that I say: switch to a real text editor.
Emacs and Neovim are both great options if you're willing to learn them. In all seriousness, the
`-no--toml` flag might be helpful for you. However, you will then have to enter your values into the
command line which I found to be a pain in the ass.
## Output
The output will be a TOML file (named `out.toml` by default) which contains trace information for
the full trace and suffix cache. 
