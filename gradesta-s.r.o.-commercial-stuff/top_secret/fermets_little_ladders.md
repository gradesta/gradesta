Fermat's Little Ladder Labyrinth
---------------------------------

For every odd number `n`, there are an infinite number of even numbers which can be obtained by doubling. I like to think of each of these sets of numbers as a Fermat ladder. The equation for such a set is $n \cdot 2^k$. Counterintuitively, ever Fermat ladder is a distinct set from every other ladder. How is this possible? I haven't quite managed to get my mind around it. It would seem to me, that there are exactly as many even numbers as odd numbers. So it is confusing that you can map an infinite number of even numbers to every odd one. I haven't yet managed to wrap my mind around this.

Anyways...

To create Fermat's Little Ladder Labyrinth, we must connect the ladders somehow into a graph, and then we can get ourselves lost in this labarynth of ladders. Let us start by defining the rules of our game:

1. You can climb down a ladder, but not up one.
2. You move between ladders by going to the bottom rung of the ladder you are on. The bottom rung is the only odd number in the ladder. Once you are on the bottom rung, multiply by a prime constant `p` and add `1`.
3. Once the game has started, you cannot change your constant `p`.

Now lets set the lay of the land before we talk about the goal of the game. Our first ladder's bottom rung is 1, the second is 3, the third is 5 and so on.  If we start on a given ladder and climb down and use rule 2 to jump to another ladder, an odd number (the bottom rung) times a prime (odd) number (our constant `p`) is an odd number. Plus 1 is an even number. That new even number will be a rung of a ladder. We can determine which rungs can be landed on using Fermat's little theorum which states that:

1. `n` is the odd number at the bottom of the ladder and `p` is our coeficient of up jumping.
2. If $ \frac{n}{p} \equiv b \text{ (mod c)} $
3. Then $ n \cdot 2 ^ {(p+b-1)} \equiv 1 \text{ (mod p)}$
4. Since when we jump from the bottom rung of a ladder to the top rung of a ladder we multiply n by p, we end up with a value which is $n \cdot p \equiv 0 \text { (mod p)}$, so adding 1 gets us a value that is $1 \text{ (mod p)}$.
5. Thus rungs who's index is $k=p+b-1$ must have incomming up jumps.
6. Furthermore, if $n \cdot 2^{k} \equiv 1 \text{ (mod p)}$, then by modular arithmatic  $n \cdot 2^{y \cdot p \cdot k} \equiv 1 \text{ (mod p)}$ so every $p$ rungs have incomming up jumps.

Now let me tell you the goal of the game:

Pick a starting position and a constant `p` where by following these rules (going down one ladder, jumping up to the next repeatedly), we will get back to where we started.

I'll give you an example of a winning starting position and `p`:

$$n=1, p=7$$

In this starting position we are at the bottom of a ladder (1 is an odd number). So we jump up by multiplying by `p` and adding 1 and we get to a rung `8`. We can then climb down this ladder: 8, 4, 2, 1. And we're back where we started.

Another, more interesting example is if we choose $$n=1, p=5$$. Here we jump to `6` and then we climb down to `3` and then we jump to `16` and back down to `1`.

Now try to find a starting position and a constant `p` which loops back on itself but does not pass through the number `1`.

This is impossible and here is why:

Let us project Fermat's ladders onto a logarithmic scale and draw it on a graph. You'll notice that the rungs of each ladder are exactly one step appart from eachother. Climbing down the ladder always subtracts a whole number in this projection from wherever we are at.


Let us now center our projection on the bottom rung of whichever ladder we decide to start on. We can do this by passing our numbers through:

$$
f(x) = log_2(x) - log(start)
$$

When we take the case of:

$$n=1, p=7$$

In our projection we get:

$$
f(1) = log_2(1) - log_2(1) = 0
$$

Then we jump up to $7\cdot 1+1$ which gets us to:

$$
f(7+1) = log_2(8) - log_2(1) = 3
$$

And then we climb down 3 rungs to where we started.

In the case of:

$$n=1, p=5$$

We start at:


$$
f(1) = log_2(1) - log_2(1) = 0
$$

We jump up to $5 \cdot 1 + 1$ which gets us to:

$$
f(5+1) = log_2(6) - log_2(1) \approx 2.584962
$$

$$
\triangle = \log_2(\frac{6}{1}) \approx 2.584962
$$

We then climb down the ladder one rung to

$$
f(3) = log_2(3) - log_2(1) \approx 1.584962
$$

We then jump up to

$$
f(15+1) = log_2(16) - log_2(1) = 4
$$

$$
\triangle = log_2(\frac{16}{3}) \approx 2.4150374
$$

And climb down 4 rungs to where we started.

Now if we rewrite the up jumps logarithmic product:

$$
log_2(\frac{6 \cdot 16}{1 \cdot 3}) = 5
$$

If we were to start at rung 3:


$$
f(3) = log_2(3) - log_2(3) = 0
$$

We jump up to $5 \cdot 3 + 1$ which gets us to:

$$
f(15+1) = log_2(16) - log_2(3) \approx 2.4150374
$$

$$
\triangle = log_2(\frac{16}{3}) \approx 2.4150374
$$

We then climb down the ladder one rung to

$$
f(1) = log_2(1) - log_2(3) \approx -1.584962
$$

We then jump up to

$$
f(5+1) = log_2(6) - log_2(3) = 1
$$

$$
\triangle = log_2(\frac{6}{3-3}) \approx 2.584962
$$

And we climb down one rung back to `0`.

In this we have more 3's in the denominator.

$$
log_2(\frac{16 \cdot 6}{3 \cdot (3 - 3)}) = 5
$$

Lets return to the equation which shows the sum of the up jumps:

$$
log_2(\frac{6 \cdot 16}{1 \cdot 3}) = 5
$$

For this to be a whole number, all non 2 factors of the denominator must cancel out with the factors in the numerator. Lets look at where these factors orignate.

In the case of

$$n=1, p=7$$


These factors simply never appear. The fraction ends up being:

$$
log_2(\frac{8}{1})
$$

In the case of


$$
log_2(\frac{6 \cdot 16}{1 \cdot 3}) = 5
$$

The 3 in the numerator comes from the first upwards jump and is imediately cancled out by the delta calculation in the second jump.

Now lets look at what happens when we try to walk the ladders from a starting point that doesn't cycle.

$$
n=5, p=5
$$

$$
f(5) = log_2(5) - log_2(5) = 0
$$

$$
f(25+1) = log_2(26) - log_2(5)
$$

$$
\triangle = log_2(\frac{26}{5})
$$

$$
f(13) = log_2(13) - log_2(5)
$$

$$
f(13 \cdot 5+1) = log_2(66) - log_2(5)
$$

$$
\triangle = log_2(\frac{66}{13+5-5})
$$


$$
f(33) = log_2(33) - log_2(5)
$$

$$
f(33 \cdot 5+1) = log_2(166) - log_2(5)
$$

$$
\triangle = log_2(\frac{166}{33+5-5})
$$

Now we could continue for a long time with this, but we already see that we've accumulated a lot of prime factors in both the numerator and the denominator. Here is what we have so far:

$$
\sum \triangle = log_2(\frac{26*66*166}{5*13*33})
$$

Or if we do a prime factorization:

$$
\sum \triangle = log_2(\frac{13*2*2*11*3*83*2}{5*13*11*3})
$$

And simplification:

$$
\sum \triangle = log_2(\frac{2*2*2*2*83}{5})
$$

After each step, we gain a factor and cancel it out, except for that pesky 5 in the denominator. Under what circumstances could that be cancled? Only if we went up by a multiple of 5. This is impossible, however, as we always go up by $1 (mod 5)$. We will never get that missing 5 which was introduced by the $- log(5)$ in the second step. The only reason why this does not occure for loops which contain a 1 is that $- log(1)$ is $0$ and thus no extraneous factor in the denominator is ever introduced.