use serde::Serialize;

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(crate) enum Event {
    Module(ModuleEvent),
    Test(TestEvent),
}

impl Event {
    pub(crate) fn module_started(module_name: String) -> Self {
        Event::Module(ModuleEvent::Started { module_name })
    }

    pub(crate) fn module_finished(module_name: String, exec_time: f64) -> Self {
        Event::Module(ModuleEvent::Finished {
            module_name,
            exec_time,
        })
    }

    pub(crate) fn test_started(fn_name: String) -> Self {
        Event::Test(TestEvent::Start { fn_name })
    }

    pub(crate) fn test_passed(fn_name: String, exec_time: f64) -> Self {
        Event::Test(TestEvent::Pass(TestOutcome {
            fn_name,
            failure: None,
            exec_time,
        }))
    }

    pub(crate) fn test_failed(fn_name: String, exec_time: f64, rendered_failure: String) -> Self {
        Event::Test(TestEvent::Fail(TestOutcome {
            fn_name,
            failure: Some(rendered_failure),
            exec_time,
        }))
    }

    pub(crate) fn test_timeout(fn_name: String, exec_time: f64, rendered_failure: String) -> Self {
        Event::Test(TestEvent::Timeout(TestOutcome {
            fn_name,
            failure: Some(rendered_failure),
            exec_time,
        }))
    }
}

#[derive(Serialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub(crate) enum ModuleEvent {
    Started { module_name: String },
    Finished { module_name: String, exec_time: f64 },
}

#[derive(Serialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub(crate) enum TestEvent {
    Start { fn_name: String },
    Pass(TestOutcome),
    Fail(TestOutcome),
    Timeout(TestOutcome),
}

#[derive(Serialize)]
pub(crate) struct TestOutcome {
    fn_name: String,
    exec_time: f64,
    failure: Option<String>,
}
