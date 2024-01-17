use ydb_grpc::ydb_proto::auth::v1::auth_service_client::AuthServiceClient;

use crate::grpc_wrapper::raw_auth_service::login::RawLoginResult;
use crate::grpc_wrapper::raw_auth_service::login::RawLoginRequest;
use crate::grpc_wrapper::runtime_interceptors::InterceptedChannel;
use crate::grpc_wrapper::raw_errors::RawResult;

use tracing::trace;

pub(crate) struct RawAuthClient {
    service: AuthServiceClient<InterceptedChannel>,
}

impl RawAuthClient {
    pub fn new(service: InterceptedChannel) -> Self {
        Self {
            service: AuthServiceClient::new(service),
        }
    }

    pub async fn login(&mut self, req: RawLoginRequest) -> RawResult<RawLoginResult> {
        request_with_result!(
            self.service.login,
            req => ydb_grpc::ydb_proto::auth::LoginRequest,
            ydb_grpc::ydb_proto::auth::LoginResult => RawLoginResult
        );
    }
}
