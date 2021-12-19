Tips for KCF programmers
-----------------------------

Writing code for a Kanban Code Flow requires you to write differently than you are probably used to.

Code needs to be broken up into descrete chunks which do not need to change over time.

OOP is incompatible
-----------------------

This means that Object Oriented Programming in which you define classes that combine data and methods is not compatible with KCF.

A class is not a descrete chunk that does not need to change over time. You can't add new functionality without adding a method, and a method is not a descrete chunk.

C style function based programming is compatible with KCF.

Well defined structs rarely need to change.

Functions that operate on those structs are descrete.

You can add new functions that operate on those structs without needing to touch the old ones.

Dependency injection, continuation passing style and first class functions can be problematic
----------------------------------------------------------------------------------------------------------------

You want your code to be discrete and fully tested.

If you have moving parts that constitute unknowns in your logic, it's hard to fully understand and fully test.

How can you be sure that your code is "complete" when you don't even know how the internals are going to function?

How can you fully test it?

This doesn't mean that dependency injection is incompatible with KCF, but it can be problematic if used wrong.

A hierarchy of functional dependencies is ideal
---------------------------------------------------------

If you can attain a hierarchy of functional dependencies, in which the functions and data structures at the top are finished and unchainging, this is an ideal state. Most of the code you depend on is well understood and will not change underneath you. You are standing on solid ground.
