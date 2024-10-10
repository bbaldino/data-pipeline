# Dev Notes

### Do we need a trait for Node?

Given that we're already defining the inner 'packet handler' types via an enum,
I'm not sure the node trait buys us much.  We had the following methods there:

name: return a string name for the node attach: attach a node after that one
get_stats: get stats from the node visit: visitor

Anything that didn't already look a lot like the current node impl that had to
implement all those methods...not sure the flexibility is buying us much there.
We only had the one impl (DefaultNode) and the packethandler enum already seems
to cover every use case (observing, filtering, transforming) so the flexibility
can come from there.

### Making the pipeline generic with regards to the data type

It'd be great to have the pipeline "framework" not be packet/RTP specific and,
instead, just be a generic set of logic for stringing together different nodes
where each node can play different roles (observer, transformer, etc.).

Surprisingly, the enum seemed to play ok with the generic, i.e.:

```rust
pub trait Observer<T> { 
  fn observe(&mut self, data: T); 
}

pub trait Transformer<T> { 
  fn transform(&mut self, data: T) -> Result<T>; 
}

pub enum SomeHandler<T> { 
  Observer(Box<dyn Observer<T>>),
  Transformer(Box<dyn Transformer<T>>), 
  ... 
} 
```

(i'd expected a trait object complaint there...maybe i just haven't gotten far
enough along yet) But, the generics will be too viral, I think.  If the handler
is generic on T, then the Node, which holds a Handler, also has to be generic.
That also means that NodeRef will have to be generic, and I think that feels
like it'd be too much.
