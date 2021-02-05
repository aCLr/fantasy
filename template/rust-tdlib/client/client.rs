use crate::{
    client::observer::OBSERVER,
    errors::{RTDError, RTDResult},
    tdjson,
    types::RFunction,
    types::*,
};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

#[doc(hidden)]
pub trait TdLibClient {
    fn send<Fnc: RFunction>(&self, client_id: tdjson::ClientId, fnc: Fnc) -> RTDResult<()>;
    fn receive(&self, timeout: f64) -> Option<String>;
    fn execute<Fnc: RFunction>(&self, fnc: Fnc) -> RTDResult<Option<String>>;
}

#[derive(Clone, Debug)]
#[doc(hidden)]
pub struct RawApi;

impl Default for RawApi {
    fn default() -> Self {
        Self
    }
}

impl TdLibClient for RawApi {
    fn send<Fnc: RFunction>(&self, client_id: tdjson::ClientId, fnc: Fnc) -> RTDResult<()> {
        let json = fnc.to_json()?;
        tdjson::send(client_id, &json[..]);
        Ok(())
    }

    fn receive(&self, timeout: f64) -> Option<String> {
        tdjson::receive(timeout)
    }

    fn execute<Fnc: RFunction>(&self, fnc: Fnc) -> RTDResult<Option<String>> {
        let json = fnc.to_json()?;
        Ok(tdjson::execute(&json[..]))
    }
}

impl RawApi {
    pub fn new() -> Self {
        Self {}
    }
}


#[derive(Debug, Clone)]
pub enum ClientState {
    /// Client opened. You can start interaction
    Opened,
    /// Client closed properly. You must reopen it if you want to interact with Telegram
    Closed,
    /// Client closed with error
    Error(String),
}

#[derive(Clone, Debug)]
/// Struct stores all methods which you can call to interact with Telegram, such as:
/// [send_message](Api::send_message), [download_file](Api::download_file), [search_chats](Api::search_chats) and so on.
pub struct Client<S>
where
    S: TdLibClient,
{
    raw_api: S,
    client_id: i32,
    updates_sender: Option<mpsc::Sender<Box<TdType>>>,
    auth_state_sender: mpsc::Sender<ClientState>,
    auth_state_receiver: Option<mpsc::Receiver<ClientState>>,
    tdlib_parameters: TdlibParameters,
}

impl<S> Client<S>
where
    S: TdLibClient,
{
    pub fn client_id(&self) -> i32 {
        self.client_id
    }
    pub fn tdlib_parameters(&self) -> &TdlibParameters {
        &self.tdlib_parameters
    }

    pub(crate) fn updates_sender(&self) -> &Option<mpsc::Sender<Box<TdType>>> {
        &self.updates_sender
    }

    pub(crate) fn auth_sender(&self) -> &mpsc::Sender<ClientState> {
        &self.auth_state_sender
    }
}

#[derive(Debug)]
pub struct ClientBuilder<R>
where
    R: TdLibClient,
{
    updates_sender: Option<mpsc::Sender<Box<TdType>>>,
    tdlib_parameters: Option<TdlibParameters>,
    tdjson: R,
}

impl Default for ClientBuilder<RawApi> {
    fn default() -> Self {
        Self {
            updates_sender: None,
            tdlib_parameters: None,
            tdjson: RawApi::new(),
        }
    }
}

impl<R> ClientBuilder<R>
where
    R: TdLibClient,
{
    /// If you want to receive real-time updates (new messages, calls, etc.) you have to receive them with tokio::mpsc::Receiver<TdType>
    pub fn with_updates_sender(mut self, updates_sender: mpsc::Sender<Box<TdType>>) -> Self {
        self.updates_sender = Some(updates_sender);
        self
    }

    /// Base parameters for your TDlib instance.
    pub fn with_tdlib_parameters(mut self, tdlib_parameters: TdlibParameters) -> Self {
        self.tdlib_parameters = Some(tdlib_parameters);
        self
    }

    pub fn with_tdjson<T: TdLibClient>(mut self, tdjson: T) -> ClientBuilder<T> {
        ClientBuilder {
            tdjson,
            updates_sender: self.updates_sender,
            tdlib_parameters: self.tdlib_parameters,
        }
    }

    pub fn build(self) -> RTDResult<Client<R>> {
        if self.tdlib_parameters.is_none() {
            return Err(RTDError::BadRequest("tdlib_parameters not set"));
        };

        let client = Client::new(
            self.tdjson,
            self.updates_sender,
            self.tdlib_parameters.unwrap(),
        );
        Ok(client)
    }
}

/// TDLib high-level API methods.
/// Methods documentation can be found in https://core.telegram.org/tdlib/docs/td__api_8h.html
impl<R> Client<R>
where
    R: TdLibClient,
{
    pub fn builder() -> ClientBuilder<RawApi> {
        ClientBuilder::default()
    }

    pub fn new(
        raw_api: R,
        updates_sender: Option<mpsc::Sender<Box<TdType>>>,
        tdlib_parameters: TdlibParameters,
    ) -> Self {
        let client_id = tdjson::new_client();
        let (auth_state_sender, auth_state_receiver) = mpsc::channel(10);
        Self {
            raw_api,
            client_id,
            auth_state_receiver: Some(auth_state_receiver),
            auth_state_sender,
            updates_sender,
            tdlib_parameters,
        }
    }

    pub async fn wait_for_auth(&self) -> RTDResult<JoinHandle<ClientState>> {
        let mut recv = match &self.auth_state_receiver {
            None => return Err(RTDError::BadRequest("client already initialized")),
            Some(recv) => {
                recv
            }
        };
        if let Some(msg) = recv.recv().await {
            match msg {
                ClientState::Closed => return Ok(tokio::spawn(async { ClientState::Closed })),
                ClientState::Error(e) => return Err(RTDError::TdlibError(e)),
                ClientState::Opened => {}
            }
        }
        Ok(tokio::spawn(async move {
             match recv.recv().await {
                Some(ClientState::Opened) => ClientState::Error("received Opened state again".to_string()),
                Some(state) => state,
                None => ClientState::Error("auth state channel closed".to_string())
            }
        }))

    }

{% for token in tokens %}{% if token.type_ == 'Function' %}
  // {{ token.description }}
  pub async fn {{token.name | to_snake}}<C: AsRef<{{token.name | to_camel}}>>(&self, {{token.name | to_snake}}: C) -> RTDResult<{{token.blood | to_camel}}> {
    let extra = {{token.name | to_snake }}.as_ref().extra()
      .ok_or(RTDError::Internal("invalid tdlib response type, not have `extra` field"))?;
    let signal = OBSERVER.subscribe(&extra);
    self.raw_api.send(self.client_id, {{token.name | to_snake }}.as_ref())?;
    let received = signal.await;
    OBSERVER.unsubscribe(&extra);
    match received {
      Err(_) => {Err(RTDError::Internal("receiver already closed"))}
      Ok(v) => match v {
        TdType::{{token.blood | to_camel}}(v) => { Ok(v) }
        {% if token.blood != "Error" %}TdType::Error(v) => { Err(RTDError::TdlibError(v.message().clone())) }{% endif %}
        _ => {
          error!("invalid response received: {:?}", v);
          Err(RTDError::Internal("receive invalid response"))
        }
      }
    }
  }
  {% endif %}{% endfor %}
}

#[cfg(test)]
mod tests {
    use crate::client::api::TdLibClient;
    use crate::client::client::{Client, ConsoleAuthStateHandler};
    use crate::errors::RTDResult;
    use crate::types::{
        Chats, RFunction, RObject, SearchPublicChats, TdlibParameters, UpdateAuthorizationState,
    };
    use std::time::Duration;
    use tokio::sync::mpsc;
    use tokio::time::timeout;

    #[derive(Clone)]
    struct MockedRawApi {
        to_receive: Option<String>,
    }

    impl MockedRawApi {
        pub fn set_to_receive(&mut self, value: String) {
            trace!("delayed to receive: {}", value);
            self.to_receive = Some(value);
        }

        pub fn new() -> Self {
            Self { to_receive: None }
        }
    }

    impl TdLibClient for MockedRawApi {
        fn send<Fnc: RFunction>(&self, _fnc: Fnc) -> RTDResult<()> {
            Ok(())
        }

        fn receive(&self, timeout: f64) -> Option<String> {
            std::thread::sleep(Duration::from_secs(timeout as u64));
            if self.to_receive.is_none() {
                panic!("value to receive not set");
            }
            self.to_receive.clone()
        }

        fn execute<Fnc: RFunction>(&self, _fnc: Fnc) -> RTDResult<Option<String>> {
            unimplemented!()
        }
    }

    #[tokio::test]
    async fn test_request_flow() {
        // here we just test request-response flow with SearchPublicChats request
        env_logger::init();
        let mut mocked_raw_api = MockedRawApi::new();

        let search_req = SearchPublicChats::builder().build();

        let chats: serde_json::Value = serde_json::from_str(
            Chats::builder()
                .chat_ids(vec![1, 2, 3])
                .build()
                .to_json()
                .unwrap()
                .as_str(),
        )
        .unwrap();
        let mut chats_object = chats.as_object().unwrap().clone();
        chats_object.insert(
            "@extra".to_string(),
            serde_json::Value::String(search_req.extra().unwrap()),
        );
        let to_receive = serde_json::to_string(&chats_object).unwrap();
        mocked_raw_api.set_to_receive(to_receive);

        let client = Client::new(
            mocked_raw_api.clone(),
            ConsoleAuthStateHandler::default(),
            TdlibParameters::builder().build(),
            None,
            2.0,
        );

        let (sx, _rx) = mpsc::channel::<UpdateAuthorizationState>(10);
        let _updates_handle = client.init_updates_task(sx);
        trace!("chats objects: {:?}", chats_object);
        match timeout(
            Duration::from_secs(10),
            client.api().search_public_chats(search_req),
        )
        .await
        {
            Err(_) => panic!("did not receive response within 1 s"),
            Ok(Err(e)) => panic!("{}", e),
            Ok(Ok(result)) => assert_eq!(result.chat_ids(), &vec![1, 2, 3]),
        }
    }
}
