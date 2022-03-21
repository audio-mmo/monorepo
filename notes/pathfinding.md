Just jotting this down so that it's somewhere not in my head.

Pathfinding has the following common cases:

- Straight line from start to a goal.
- Mostly straight line from start to a goal, but going around obstacles.
-  Getting to the goal is complicated, and involves going to tiles further away before we get there.

We usually care about a reasonable path, not the most minimal path.

Problems:

- Astar is slow and takes too much memory.
- IDA* is O(1) on memory, but way too slow.
- Neither of these actually necessarily handle the common case quickly.

So let's consider these optimizations, , where each step is bounded:

- Run a simple algorithm which just tries always moving closer.
- Run IDA* for some number of iterations, but modified to sort successors by least cost first.

From college, for a problem in which I had astar over a search space that was way too large, the following formulation
of the cost function proved beneficial:

```
h(n)=w*f(path)+(1-w)*g(successor)
```

Where "normal" astar is `w=0.5`.  This has the interesting property that tuning w tunes things from bfs to astar; in
particular just using the heuristic can sometimes lead to convoluted paths but surprisingly quickly.