// use tracing_subscriber::Layer;
//
// struct VisualLayer{
//
// }
//
// impl<S> Layer<S> for VisualLayer
//     where S: tracing::Subscriber,
//     Self: 'static,
// {
//     fn on_event(&self, event: &tracing::Event<'_>, ctx: tracing_subscriber::layer::Context<'_, S>) {
//         println!("Event: {:?}", event);
//     }
//
//     fn on_new_span(&self, attrs: &tracing::span::Attributes<'_>, id: &tracing::span::Id, ctx: tracing_subscriber::layer::Context<'_, S>) {
//         println!("New Span: {:?}", attrs);
//     }
//
//     fn on_enter(&self, id: &tracing::span::Id, ctx: tracing_subscriber::layer::Context<'_, S>) {
//         println!("Enter: {:?}", id);
//     }
//
//     fn enabled(&self, metadata: &tracing::Metadata<'_>, ctx: tracing_subscriber::layer::Context<'_, S>) -> bool {
//         println!("Enabled: {:?}", metadata);
//         true
//     }
//
// }

// TODO On the simulation side of things we will need the messages from machines to be processed and sent in batches

// TODO Create some method to read in the log file and create a viz from it log files should begin with the ndl that created the sim
// ensure that playback doesn't create logs, logs should maybe be created at a different level of the hierarchy 

// TODO Create vizMachines from ndl

//TODO Create a message handler that will take and distribute messages from the sim to vizMachines
// Messages from sim should come in at a regular interval and should be in batches so we can do batch updates
// This should also direct messages that need to be logged somewhere (like that log file)
struct VisualHandler{

}

//TODO Log levels for each machine needs to come from ndl
enum VisualLevel {
}

//TODO Mirror simMachines, contain the information needed to visualize and receive messages from handler
// #[derive(Debug)]
// struct VisualMachine{
//     field: Type
// }


