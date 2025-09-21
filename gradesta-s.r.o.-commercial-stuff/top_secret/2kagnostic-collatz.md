## Multiplication by 3 in $2^k$ agnostic arithmatic

Since trailing zeros are ignored and multiplication by two represents a shift to the left by one digit, multiplication by two does not exist in $2^k$ agnostic arithmatic. It is, however possible to mulitply by three.

Imagine a situation where you have:

`101 * 11` That is 5 times 3.

You can do this simply by doing:

```
 101
 101
+101
----
```

The first two numbers when added together end up being the equivalent of a left shift so you end up with:

```
1010
+101
----
1111
```

Unlike multiplication by two, multiplication by 3 makes sense in this form of arithmatic.

Multiplication by 3 is the same as adding either a right shifted or left shifted version of the number to the number. It really doesn't matter which direction you shift it as the result will be the same either way.

Like it can either be:

```
 101
+ 101
```

Or

```
  101
+101
```

So that's multiplication by 3. In order to represent the Collatz tranformation we also need to add 1. But this works exactly as it does in ordinary binary. It's just important to make sure you line up the least significant digits.

You can transition between various walktrees in the condensed Collatz graph by doing 3n+1 in $2^k$ agnostic arithmatic.

```
 101
  101
+   1
-----
1
```

This represents the transition between 5 and 1. Note the lack of trailing zeros.

5 happens to be just one step from one but the sequence does tend to be longer consisting of multiple iterations of "add shift right" and +1.

If we place these binary sequences on a grid and run just the the "add shift right" operation over and over again we get something interesting. A crystaline structure of 1's and zeros with larger crystals of triangular shape sometimes included. Various crystaline and "metalic" structures appear depending on the initial binary sequence that that we enter.

If we then add the +1 operation, we see that this crystaline structure gets eaten or cut from the right to the left, except when we come across a larger dark triangular crystal at which point the structure is cut verically untill the dark triangle is consumed.

The only time we cut vertically is when we are cutting through dark crystals. Otherwise we are cutting sideways. The height of the cut crystals is always the same as their width.

We also see the crystaline structure growing slowly in the rightward direction. Basically, the whole conjecture comes down to if this cutting is faster than that growth.

We notice that adding new bits on the right to our initial sequence doesn't really effect things on the left unless there are carries. If there are carries crystals form. The possibility of a proof by induction that starts with a small sequence and grows it rightward occures to me, but there is a much easier proof of collatz.

It turns out that if you draw a vertical line in the grid where the initial 1 ocures. Then all the leftward crystal growth comes from one of two effects:

- carries
- add shift rights

Now there is at most one carry per iteration, and the add shift rights are equivalent to adding a given subsequence divided by two (rounded up). Thus the maximum binary value of each binary subsequence to the right of our imaginary virtual line is 

```
  1+1/2 (cumulatively 1)
  1+1/2 (cumulatively 2)
  1+2/2 (cumulatively 4)
  1+4/2 (cumulatively 7)
  1+7/2 (cumulatively 11)
 ...
 ```

 Which is equivalent to the subsequence to the left of our imaginary vertical line being equal to at most $3(1.5)^i-2$. We will refer to this value as $n_1$ and the value of our initial binary sequence as $n_0$.

 Now as we are cutting to the left by at least one colum any time we are not in a dark crystal, and between any dark crystals. We can calculate the maximum number of iterations before we reach this subsequence. Which is the maximum size of a dark crystal times the number of columns (which is equal to the number of bits in our initial sequence). I'm going to refer to the number of bits in our initial sequence as $l$ and for now lets just assume that the maximum crystal size is $l$ (it could be larger, up to $l+i$ but for now lets go with this reasonable estimate). If the maximum crystal size is $l$ and we end up spending $l$ iterations cutting vertically through a crystal of size $l$ then we will spend $l*l$ iterations to cut through the maximum number of crystals. Given a worse case scenario that the crystals are 1 colum appart, which is impossibly pessimistic.

 If we go with this estimate. Then the maximum value of $n_1$ is $3(1.5)^{l*l}-2$.
