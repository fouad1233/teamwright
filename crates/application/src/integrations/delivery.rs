//! Passes the events of a project on to its Slack channel and to the web
//! addresses that listen, as the manager chose on this computer.
//!
//! Delivering is slow and may fail: the shell calls [`DeliveryService::deliver`]
//! on a thread of its own, and a failure never reaches the work. It is kept
//! in memory, with the last deliveries of each address, for the
//! integrations page to show.

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex, PoisonError};

use teamwright_domain::{EventKind, Project, ProjectId, Timestamp, WebAddress};

use super::messages::{
    EventPayload, channel_message_of, channel_test_message, kind_of, payload_of, test_payload,
};
use super::{IntegrationStore, ProjectIntegrations};
use crate::error::ApplicationError;
use crate::ports::{Clock, ProjectRepository};
use crate::project_events::ProjectEvent;

/// How many deliveries of each address are kept.
pub const DELIVERIES_KEPT: usize = 10;

/// Sends to the web. Every call has a time limit. A failure is said in
/// plain words and never repeats the address.
pub trait WebSender: Send + Sync {
    /// Posts one line to a chat channel's incoming webhook. Answers with
    /// the status of the answer.
    ///
    /// # Errors
    ///
    /// Why nothing was answered: the address could not be reached, or it
    /// did not answer in time.
    fn send_to_channel(&self, address: &WebAddress, text: &str) -> Result<u16, String>;

    /// Posts an event as JSON, signed with the secret if there is one.
    /// Answers with the status of the answer.
    ///
    /// # Errors
    ///
    /// Why nothing was answered.
    fn send_event(
        &self,
        address: &WebAddress,
        secret: Option<&str>,
        payload: &EventPayload,
    ) -> Result<u16, String>;
}

/// One delivery to an address, or to the channel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Delivery {
    /// When it was sent.
    pub at: Timestamp,
    /// The name of the event, or `test`.
    pub event: &'static str,
    /// The status of the answer, or why there was none.
    pub outcome: Result<u16, String>,
}

impl Delivery {
    /// Whether the address took it: an answer in the 200s.
    #[must_use]
    pub fn succeeded(&self) -> bool {
        matches!(self.outcome, Ok(status) if (200..300).contains(&status))
    }

    /// Why it was not taken, in words, or nothing if it was.
    #[must_use]
    pub fn failure(&self) -> Option<String> {
        match &self.outcome {
            Ok(_) if self.succeeded() => None,
            Ok(status) => Some(format!("the address answered {status}")),
            Err(reason) => Some(reason.clone()),
        }
    }
}

/// What was delivered for a project since the program started.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Deliveries {
    /// The last message to the channel that could not be delivered, if
    /// the last one failed.
    pub channel_failure: Option<Delivery>,
    /// For each web address of the project, in their order, its last
    /// deliveries, the newest first.
    pub addresses: Vec<Vec<Delivery>>,
}

/// What the delivery needs.
#[derive(Clone)]
pub struct DeliveryPorts {
    /// The projects, for their keys.
    pub projects: Arc<dyn ProjectRepository>,
    /// The settings of the integrations.
    pub store: Arc<dyn IntegrationStore>,
    /// The web.
    pub sender: Arc<dyn WebSender>,
    /// The time.
    pub clock: Arc<dyn Clock>,
}

#[derive(Default)]
struct Memory {
    channel_failures: HashMap<ProjectId, Delivery>,
    /// Keyed by the project and the address itself, so that the deliveries
    /// stay with an address when another one is removed.
    deliveries: HashMap<(ProjectId, String), VecDeque<Delivery>>,
}

/// Passes events on to the channel and the addresses of their project.
#[derive(Clone)]
pub struct DeliveryService {
    ports: DeliveryPorts,
    memory: Arc<Mutex<Memory>>,
}

impl DeliveryService {
    /// Creates the service, with nothing delivered yet.
    #[must_use]
    pub fn new(ports: DeliveryPorts) -> Self {
        Self {
            ports,
            memory: Arc::new(Mutex::new(Memory::default())),
        }
    }

    /// Passes an event on to wherever the manager chose. It waits for the
    /// answers, so it belongs on a thread of its own. Nothing is reported:
    /// what failed is kept for the integrations page.
    pub fn deliver(&self, event: &ProjectEvent) {
        // Settings or a project that cannot be read send nothing.
        let Ok(integrations) = self.ports.store.integrations(event.project_id) else {
            return;
        };
        if integrations.slack.is_none() && integrations.web_addresses.is_empty() {
            return;
        }
        let Ok(Some(project)) = self.ports.projects.find(event.project_id) else {
            return;
        };
        let kind = kind_of(&event.happening);
        for target in targets_of(&integrations, kind) {
            match target {
                Target::Channel(address) => {
                    let outcome = self
                        .ports
                        .sender
                        .send_to_channel(address, &channel_message_of(event));
                    self.keep_channel(project.id(), kind.as_str(), outcome);
                }
                Target::Address { address, secret } => {
                    let payload = payload_of(event, project.key().as_str());
                    let outcome = self.ports.sender.send_event(address, secret, &payload);
                    self.keep(project.id(), address, payload.event, outcome);
                }
            }
        }
    }

    /// Sends a line to the project's channel to try it, and answers with
    /// what came of it.
    ///
    /// # Errors
    ///
    /// [`ApplicationError::NoChannel`] if no channel is set; an error if
    /// there is no such project, or if storage fails.
    pub fn test_channel(&self, project_id: ProjectId) -> Result<Delivery, ApplicationError> {
        let project = self.project(project_id)?;
        let slack = self
            .ports
            .store
            .integrations(project_id)?
            .slack
            .ok_or(ApplicationError::NoChannel)?;
        let outcome = self.ports.sender.send_to_channel(
            &slack.address,
            &channel_test_message(project.name().as_str()),
        );
        Ok(self.keep_channel(project_id, "test", outcome))
    }

    /// Sends the event `test` to the web address at a position, and
    /// answers with what came of it.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no address there, if there is no such
    /// project, or if storage fails.
    pub fn test_address(
        &self,
        project_id: ProjectId,
        index: usize,
    ) -> Result<Delivery, ApplicationError> {
        let project = self.project(project_id)?;
        let integrations = self.ports.store.integrations(project_id)?;
        let listening =
            integrations
                .web_addresses
                .get(index)
                .ok_or_else(|| ApplicationError::NotFound {
                    kind: "web address",
                    id: (index + 1).to_string(),
                })?;
        let payload = test_payload(
            project_id,
            project.key().as_str(),
            project.name().as_str(),
            self.ports.clock.now(),
        );
        let outcome =
            self.ports
                .sender
                .send_event(&listening.address, listening.secret.as_deref(), &payload);
        Ok(self.keep(project_id, &listening.address, payload.event, outcome))
    }

    /// What was delivered for a project since the program started.
    ///
    /// # Errors
    ///
    /// Returns an error if storage fails.
    pub fn deliveries(&self, project_id: ProjectId) -> Result<Deliveries, ApplicationError> {
        let integrations = self.ports.store.integrations(project_id)?;
        let memory = self.memory.lock().unwrap_or_else(PoisonError::into_inner);
        Ok(Deliveries {
            channel_failure: memory.channel_failures.get(&project_id).cloned(),
            addresses: integrations
                .web_addresses
                .iter()
                .map(|listening| {
                    memory
                        .deliveries
                        .get(&(project_id, listening.address.as_str().to_owned()))
                        .map(|kept| kept.iter().cloned().collect())
                        .unwrap_or_default()
                })
                .collect(),
        })
    }

    fn keep_channel(
        &self,
        project_id: ProjectId,
        event: &'static str,
        outcome: Result<u16, String>,
    ) -> Delivery {
        let delivery = Delivery {
            at: self.ports.clock.now(),
            event,
            outcome,
        };
        let mut memory = self.memory.lock().unwrap_or_else(PoisonError::into_inner);
        if delivery.succeeded() {
            memory.channel_failures.remove(&project_id);
        } else {
            memory.channel_failures.insert(project_id, delivery.clone());
        }
        delivery
    }

    fn keep(
        &self,
        project_id: ProjectId,
        address: &WebAddress,
        event: &'static str,
        outcome: Result<u16, String>,
    ) -> Delivery {
        let delivery = Delivery {
            at: self.ports.clock.now(),
            event,
            outcome,
        };
        let mut memory = self.memory.lock().unwrap_or_else(PoisonError::into_inner);
        let kept = memory
            .deliveries
            .entry((project_id, address.as_str().to_owned()))
            .or_default();
        kept.push_front(delivery.clone());
        kept.truncate(DELIVERIES_KEPT);
        delivery
    }

    fn project(&self, id: ProjectId) -> Result<Project, ApplicationError> {
        self.ports
            .projects
            .find(id)?
            .ok_or_else(|| ApplicationError::NotFound {
                kind: "project",
                id: id.to_string(),
            })
    }
}

/// Where an event goes.
#[derive(Debug, PartialEq, Eq)]
pub enum Target<'settings> {
    /// The Slack channel.
    Channel(&'settings WebAddress),
    /// A web address, with its secret.
    Address {
        /// The address.
        address: &'settings WebAddress,
        /// The secret the body is signed with, if any.
        secret: Option<&'settings str>,
    },
}

/// Where an event of a kind goes with the settings of its project: the
/// channel if it tells that kind, and every address that hears it.
#[must_use]
pub fn targets_of(integrations: &ProjectIntegrations, kind: EventKind) -> Vec<Target<'_>> {
    let mut targets = Vec::new();
    if let Some(slack) = &integrations.slack
        && slack.events.contains(&kind)
    {
        targets.push(Target::Channel(&slack.address));
    }
    for listening in &integrations.web_addresses {
        if listening.events.contains(&kind) {
            targets.push(Target::Address {
                address: &listening.address,
                secret: listening.secret.as_deref(),
            });
        }
    }
    targets
}

#[cfg(test)]
mod tests {
    use teamwright_domain::{ProjectKey, ProjectName, SprintId, Workflow};

    use super::*;
    use crate::SystemClock;
    use crate::in_memory::InMemoryProjectRepository;
    use crate::integrations::tests::MemoryStore;
    use crate::integrations::{IntegrationSettings, ListeningAddress, SlackChannel};
    use crate::project_events::Happening;

    /// Records what would have been sent; the address `https://down.example`
    /// cannot be reached.
    #[derive(Default)]
    struct RecordingSender {
        sent: Mutex<Vec<(String, String)>>,
    }

    impl WebSender for RecordingSender {
        fn send_to_channel(&self, address: &WebAddress, text: &str) -> Result<u16, String> {
            self.answer(address, text.to_owned())
        }

        fn send_event(
            &self,
            address: &WebAddress,
            secret: Option<&str>,
            payload: &EventPayload,
        ) -> Result<u16, String> {
            self.answer(
                address,
                format!("{} signed {}", payload.event, secret.is_some()),
            )
        }
    }

    impl RecordingSender {
        fn answer(&self, address: &WebAddress, what: String) -> Result<u16, String> {
            if address.as_str().starts_with("https://down.") {
                return Err("the address could not be reached".to_owned());
            }
            self.sent
                .lock()
                .unwrap()
                .push((address.as_str().to_owned(), what));
            Ok(200)
        }
    }

    fn address(text: &str) -> WebAddress {
        WebAddress::parse(text, "it").unwrap()
    }

    fn setting() -> (
        DeliveryService,
        IntegrationSettings,
        Arc<RecordingSender>,
        Project,
    ) {
        let projects = Arc::new(InMemoryProjectRepository::default());
        let project = Project::create(
            ProjectName::parse("Smart sensor node").unwrap(),
            ProjectKey::parse("SH").unwrap(),
            String::new(),
            Workflow::standard(),
            chrono::Utc::now(),
        );
        projects.save(&project).unwrap();
        let store = Arc::new(MemoryStore::default());
        let sender = Arc::new(RecordingSender::default());
        let service = DeliveryService::new(DeliveryPorts {
            projects: projects.clone(),
            store: store.clone(),
            sender: sender.clone(),
            clock: Arc::new(SystemClock),
        });
        (
            service,
            IntegrationSettings::new(projects, store),
            sender,
            project,
        )
    }

    fn started(project: &Project) -> ProjectEvent {
        ProjectEvent {
            project_id: project.id(),
            project_name: project.name().as_str().to_owned(),
            at: chrono::Utc::now(),
            happening: Happening::SprintStarted {
                sprint_id: SprintId::generate(),
                name: "Sprint 2".to_owned(),
                goal: String::new(),
            },
        }
    }

    #[test]
    fn an_event_goes_where_its_kind_is_chosen() {
        let integrations = ProjectIntegrations {
            repository: None,
            slack: Some(SlackChannel {
                address: address("https://hooks.slack.com/x"),
                events: [EventKind::TaskFinished].into_iter().collect(),
            }),
            web_addresses: vec![
                ListeningAddress {
                    address: address("https://one.example/"),
                    secret: Some("s".to_owned()),
                    events: [EventKind::TaskFinished, EventKind::TaskCreated]
                        .into_iter()
                        .collect(),
                },
                ListeningAddress {
                    address: address("https://two.example/"),
                    secret: None,
                    events: [EventKind::TaskCreated].into_iter().collect(),
                },
            ],
        };
        assert_eq!(targets_of(&integrations, EventKind::TaskFinished).len(), 2);
        assert_eq!(
            targets_of(&integrations, EventKind::TaskCreated),
            vec![
                Target::Address {
                    address: &integrations.web_addresses[0].address,
                    secret: Some("s")
                },
                Target::Address {
                    address: &integrations.web_addresses[1].address,
                    secret: None
                },
            ]
        );
        assert!(targets_of(&integrations, EventKind::SprintStarted).is_empty());
    }

    #[test]
    fn an_event_is_delivered_and_a_failure_is_kept() {
        let (service, settings, sender, project) = setting();
        settings
            .set_slack_address(project.id(), Some("https://down.slack.example/x"))
            .unwrap();
        settings
            .add_web_address(project.id(), "https://listener.example/hook", Some("s3"))
            .unwrap();

        service.deliver(&started(&project));

        let sent = sender.sent.lock().unwrap().clone();
        assert_eq!(
            sent,
            vec![(
                "https://listener.example/hook".to_owned(),
                "sprint.started signed true".to_owned()
            )]
        );
        let deliveries = service.deliveries(project.id()).unwrap();
        let failure = deliveries.channel_failure.unwrap();
        assert_eq!(
            failure.failure().as_deref(),
            Some("the address could not be reached")
        );
        assert_eq!(deliveries.addresses.len(), 1);
        assert!(deliveries.addresses[0][0].succeeded());
    }

    #[test]
    fn the_last_ten_deliveries_of_an_address_are_kept_newest_first() {
        let (service, settings, _, project) = setting();
        settings
            .add_web_address(project.id(), "https://listener.example/hook", None)
            .unwrap();
        for _ in 0..12 {
            service.deliver(&started(&project));
        }
        let test = service.test_address(project.id(), 0).unwrap();
        assert_eq!(test.event, "test");
        let kept = &service.deliveries(project.id()).unwrap().addresses[0];
        assert_eq!(kept.len(), DELIVERIES_KEPT);
        assert_eq!(kept[0].event, "test");
    }

    #[test]
    fn a_channel_is_tried_only_when_one_is_set() {
        let (service, settings, sender, project) = setting();
        assert!(matches!(
            service.test_channel(project.id()),
            Err(ApplicationError::NoChannel)
        ));
        settings
            .set_slack_address(project.id(), Some("https://hooks.slack.com/x"))
            .unwrap();
        assert!(service.test_channel(project.id()).unwrap().succeeded());
        assert!(
            sender.sent.lock().unwrap()[0]
                .1
                .starts_with("Smart sensor node: Teamwright will tell")
        );
    }
}
