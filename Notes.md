Do we need a trait for Node?

Given that we're already defining the inner 'packet handler' types via an enum,
I'm not sure the node trait buys us much.  We had the following methods there:

name: return a string name for the node attach: attach a node after that one
get_stats: get stats from the node visit: visitor

Anything that didn't already look a lot like the current node impl that had to
implement all those methods...not sure the flexibility is buying us much there.
We only had the one impl (DefaultNode) and the packethandler enum already seems
to cover every use case (observing, filtering, transforming) so the flexibility
can come from there.
