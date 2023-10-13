use tracing_subscriber::Layer;
use tracing::field::{
    Visit,
    Field,
};

//TODO(carsonhenrich) Log levels for each machine needs to come from ndl
//TODO(carsonhenrich) Use VisualLogLevels to 
#[derive(Debug)]
enum VisualLogLevel {
}

// TODO(carsonhenrich) Create some method to read in the log file and create a vis from it log files could begin with the ndl that created the sim
// ensure that playback doesn't create logs

// TODO(carsonhenrich) Create visMachines from ndl
//TODO(carsonhenrich) Mirror simMachines, contain the information needed to visualize and receive messages from handler
#[derive(Debug)]
struct VisualMachine{
}

// TODO(carsonhenrich) Figure out how to handle events with VisualLayer
#[derive(Debug)]
/// VisualLayer is the subscriber layer that will send events to visualization during 
/// live visualization.
struct VisualLayer{
}

#[allow(unused_variables)]
impl<S> Layer<S> for VisualLayer
    where S: tracing::Subscriber,
    Self: 'static,
{
    fn on_event(&self, event: &tracing::Event<'_>, ctx: tracing_subscriber::layer::Context<'_, S>) {
        todo!();
    }

    fn on_new_span(&self, attrs: &tracing::span::Attributes<'_>, id: &tracing::span::Id, ctx: tracing_subscriber::layer::Context<'_, S>) {
        // NOTE We could use spans to filter out messages based on level of the hierarchy (ie. Network, Machine, Application, Protocol)
        todo!();
    }

    fn on_enter(&self, id: &tracing::span::Id, ctx: tracing_subscriber::layer::Context<'_, S>) {
        todo!();
    }

    fn enabled(&self, metadata: &tracing::Metadata<'_>, ctx: tracing_subscriber::layer::Context<'_, S>) -> bool {
        todo!();
    }
}

// TODO(carsonhenrich) Create a visitor for the live visualization to store tracing metadata
/// VisualVisitor is responsible for storing the fields given in a logging message as 
/// tracing will not do this automatically.
#[derive(Debug)]
struct VisualVisitor {
}


#[allow(unused_variables)]
impl Visit for VisualVisitor {
    // NOTE(carsonhenrich) this function would allow us to log structured data but 
    // it requires tracing-unstable and the valuable crate
    // fn record_value(&mut self, field: &Field, value: &dyn Visit) {
    // }

    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        todo!()
    }
}
