# data-pipeline

This crate has utilities for connecting different types of pipeline nodes
together where each can perform an operation on some data.  It includes
facilities for building pipelines of nodes and enforcing different kinds of
access for different handlers.

## Nodes

The core element is a `Node`.  A node has a name (for user-friendliness when
gathering stats, etc.), a data handler, an optional next `Node` and an optional
previous `Node`.  It also gathers statistics about the data that has passed
through it.  The `Node` type is responsible for managing the flow of data, but
does not operate on the data in any way.

By default, nodes track a number of stats:

* The number of data items received by the node
* The number of data items forwarded by the node
* The number of data items discarded by the node
* The number of times a data handler had an error while processing data
* How much time, in total, the data handler has spent processing the data

## Data handlers

Inside every `Node` is a data handler which will operate on the data in some
way.  There are different categories of data handlers:

* Observers: observers observe data passing through but cannot change the data
in any way
* Tranformers: transformers may transform the value of the data in some way
* Filters: filters cannot change data, but decide whether a piece of data
should continue through the pipeline
* Consumers: consumers consume the data in some way and act as a termination
point for a pipeline
* Demuxers: demuxers channel data into one of possibly multiple paths based on
predicates

Each data handler category has a corresponding trait that can be implemented by
your types to perform some operation.

Data handlers can define their own stats that can be tracked in addition to the
ones tracked by `Node`.

<details>
<summary>Writing a data handler</summary>

  To write a data handler:

  1. Define your type
  1. Implement the corresponding handler trait
  1. Implement `Into<SomeDataHandler>` for your type (or use the macro)

  <details>
  <summary>Observer example</summary>
    A node that logs all data items:

  ```rust

    // The handler type
    #[derive(Default)]
    struct DataLogger {
        num_logs: u32,
    }

    // Implement the `DataObserver` trait: we just need to observe the data,
    // not change it
    impl<T> DataObserver<T> for DataLogger
    where
        T: Debug,
    {
        fn observe(&mut self, data: &T) {
            self.num_logs += 1;
            println!("{data:?}")
        }

        // Only need to implement this if your node has its own stats
        fn get_stats(&self) -> Option<serde_json::Value> {
            Some(json!({
                "num_logs": self.num_logs,
            }))
        }
    }

    // Enable conversion of your type into the `SomeDataHandler` enum
    impl<T> Into<SomeDataHandler<T>> for DataLogger
    where
        T: Debug,
    {
        fn into(self) -> SomeDataHandler<T> {
            SomeDataHandler::Observer(Box::new(self))
        }
    }
  ```

  </details>
  <details>
    <summary>Transformer example</summary>
    A node which doubles all values

  ```rust
  // The handler type
  struct DataDoubler;

  // Implement the `DataTransformer` trait since we want to change the value.
  // Here we're just going to operate on u32s
  impl DataObserver<u32> for DataDoubler
  {
    fn transform(&mut self, data: u32) -> Result<T> {
      Ok(data * 2)
    }
  }

  // Enable conversion of your type into the `SomeDataHandler` enum
  impl Into<SomeDataHandler<u32>> for DataDoubler
  {
      fn into(self) -> SomeDataHandler<T> {
          SomeDataHandler::Transformer(Box::new(self))
      }
  }
  ```

  </details>

</details>

## Pipelines

A pipeline is a set of `Node`s that are connected together.  Pipelines do not
exist as a "persistent" type, but the `PipelineBuilder` does make it easy to
connect `Node`s together.  The output of the `PipelineBuilder::build` method is
the first `Node` of the pipeline.

#### Pipeline building example

```rust
let pipeline = PipelineBuilder::new()
    .demux(
        "odd/even demuxer",
        StaticDemuxer::new(vec![
            ConditionalPath {
                predicate: Box::new(|num: &u32| num % 2 == 0),
                next: PipelineBuilder::new()
                    .attach(Node::new("1a", DataLogger))
                    .attach(Node::new("2a", DataLogger))
                    .attach(Node::new("3a", DataLogger))
                    .build(),
            },
            ConditionalPath {
                predicate: Box::new(|num: &u32| num % 2 == 1),
                next: PipelineBuilder::new()
                    .attach(Node::new("1b", DataLogger))
                    .attach(Node::new("2b", DataLogger))
                    .attach(Node::new("3b", DataLogger))
                    .build(),
            },
        ]),
    )
    .build();
```

## Visitors

Nodes support the concept of "visitors" to allow walking a pipeline and collecting information, like stats.  The `StatsNodeVisitor` type is already defined, but custom visitors could be written as well.
