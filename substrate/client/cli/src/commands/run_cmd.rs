// This file is part of Substrate.

// Copyright (C) Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

use crate::{
	arg_enums::{Cors, RpcMethods},
	error::{Error, Result},
	params::{
		ImportParams, KeystoreParams, NetworkParams, OffchainWorkerParams, RpcEndpoint,
		SharedParams, TransactionPoolParams,
	},
	CliConfiguration, PrometheusParams, RuntimeParams, TelemetryParams,
	RPC_DEFAULT_MAX_CONNECTIONS, RPC_DEFAULT_MAX_REQUEST_SIZE_MB, RPC_DEFAULT_MAX_RESPONSE_SIZE_MB,
	RPC_DEFAULT_MAX_SUBS_PER_CONN, RPC_DEFAULT_MESSAGE_CAPACITY_PER_CONN,
};
use clap::Parser;
use regex::Regex;
use sc_service::{
	config::{
		BasePath, IpNetwork, PrometheusConfig, RpcBatchRequestConfig, TransactionPoolOptions,
	},
	ChainSpec, Role,
};
use sc_telemetry::TelemetryEndpoints;
use std::{
	net::{Ipv4Addr, Ipv6Addr, SocketAddr},
	num::NonZeroU32,
};

/// The `run` command used to run a node.
#[derive(Debug, Clone, Parser)]
pub struct RunCmd {
	/// Enable validator mode.
	///
	/// The node will be started with the authority role and actively
	/// participate in any consensus task that it can (e.g. depending on
	/// availability of local keys).
	#[arg(long)]
	pub validator: bool,

	/// Disable GRANDPA.
	///
	/// Disables voter when running in validator mode, otherwise disable the GRANDPA
	/// observer.
	#[arg(long)]
	pub no_grandpa: bool,

	/// Listen to all RPC interfaces (default: local).
	///
	/// Not all RPC methods are safe to be exposed publicly.
	///
	/// Use an RPC proxy server to filter out dangerous methods. More details:
	/// <https://docs.substrate.io/build/remote-procedure-calls/#public-rpc-interfaces>.
	///
	/// Use `--unsafe-rpc-external` to suppress the warning if you understand the risks.
	#[arg(long)]
	pub rpc_external: bool,

	/// Listen to all RPC interfaces.
	///
	/// Same as `--rpc-external`.
	#[arg(long)]
	pub unsafe_rpc_external: bool,

	/// RPC methods to expose.
	#[arg(
		long,
		value_name = "METHOD SET",
		value_enum,
		ignore_case = true,
		default_value_t = RpcMethods::Auto,
		verbatim_doc_comment
	)]
	pub rpc_methods: RpcMethods,

	/// RPC rate limiting (calls/minute) for each connection.
	///
	/// This is disabled by default.
	///
	/// For example `--rpc-rate-limit 10` will maximum allow
	/// 10 calls per minute per connection.
	#[arg(long)]
	pub rpc_rate_limit: Option<NonZeroU32>,

	/// Disable RPC rate limiting for certain ip addresses.
	///
	/// Each IP address must be in CIDR notation such as `1.2.3.4/24`.
	#[arg(long, num_args = 1..)]
	pub rpc_rate_limit_whitelisted_ips: Vec<IpNetwork>,

	/// Trust proxy headers for disable rate limiting.
	///
	/// By default the rpc server will not trust headers such `X-Real-IP`, `X-Forwarded-For` and
	/// `Forwarded` and this option will make the rpc server to trust these headers.
	///
	/// For instance this may be secure if the rpc server is behind a reverse proxy and that the
	/// proxy always sets these headers.
	#[arg(long)]
	pub rpc_rate_limit_trust_proxy_headers: bool,

	/// Set the maximum RPC request payload size for both HTTP and WS in megabytes.
	#[arg(long, default_value_t = RPC_DEFAULT_MAX_REQUEST_SIZE_MB)]
	pub rpc_max_request_size: u32,

	/// Set the maximum RPC response payload size for both HTTP and WS in megabytes.
	#[arg(long, default_value_t = RPC_DEFAULT_MAX_RESPONSE_SIZE_MB)]
	pub rpc_max_response_size: u32,

	/// Set the maximum concurrent subscriptions per connection.
	#[arg(long, default_value_t = RPC_DEFAULT_MAX_SUBS_PER_CONN)]
	pub rpc_max_subscriptions_per_connection: u32,

	/// Specify JSON-RPC server TCP port.
	#[arg(long, value_name = "PORT")]
	pub rpc_port: Option<u16>,

	/// EXPERIMENTAL: Specify the JSON-RPC server interface and this option which can be enabled
	/// several times if you want expose several RPC interfaces with different configurations.
	///
	/// The format for this option is:
	/// `--experimental-rpc-endpoint" listen-addr=<ip:port>,<key=value>,..."` where each option is
	/// separated by a comma and `listen-addr` is the only required param.
	///
	/// The following options are available:
	///  • listen-addr: The socket address (ip:port) to listen on. Be careful to not expose the
	///    server to the public internet unless you know what you're doing. (required)
	///  • disable-batch-requests: Disable batch requests (optional)
	///  • max-connections: The maximum number of concurrent connections that the server will
	///    accept (optional)
	///  • max-request-size: The maximum size of a request body in megabytes (optional)
	///  • max-response-size: The maximum size of a response body in megabytes (optional)
	///  • max-subscriptions-per-connection: The maximum number of subscriptions per connection
	///    (optional)
	///  • max-buffer-capacity-per-connection: The maximum buffer capacity per connection
	///    (optional)
	///  • max-batch-request-len: The maximum number of requests in a batch (optional)
	///  • cors: The CORS allowed origins, this can enabled more than once (optional)
	///  • methods: Which RPC methods to allow, valid values are "safe", "unsafe" and "auto"
	///    (optional)
	///  • optional: If the listen address is optional i.e the interface is not required to be
	///    available For example this may be useful if some platforms doesn't support ipv6
	///    (optional)
	///  • rate-limit: The rate limit in calls per minute for each connection (optional)
	///  • rate-limit-trust-proxy-headers: Trust proxy headers for disable rate limiting (optional)
	///  • rate-limit-whitelisted-ips: Disable rate limiting for certain ip addresses, this can be
	/// enabled more than once (optional)  • retry-random-port: If the port is already in use,
	/// retry with a random port (optional)
	///
	/// Use with care, this flag is unstable and subject to change.
	#[arg(
		long,
		num_args = 1..,
		verbatim_doc_comment,
		conflicts_with_all = &["rpc_external", "unsafe_rpc_external", "rpc_port", "rpc_cors", "rpc_rate_limit_trust_proxy_headers", "rpc_rate_limit", "rpc_rate_limit_whitelisted_ips", "rpc_message_buffer_capacity_per_connection", "rpc_disable_batch_requests", "rpc_max_subscriptions_per_connection", "rpc_max_request_size", "rpc_max_response_size"]
	)]
	pub experimental_rpc_endpoint: Vec<RpcEndpoint>,

	/// Maximum number of RPC server connections.
	#[arg(long, value_name = "COUNT", default_value_t = RPC_DEFAULT_MAX_CONNECTIONS)]
	pub rpc_max_connections: u32,

	/// The number of messages the RPC server is allowed to keep in memory.
	///
	/// If the buffer becomes full then the server will not process
	/// new messages until the connected client start reading the
	/// underlying messages.
	///
	/// This applies per connection which includes both
	/// JSON-RPC methods calls and subscriptions.
	#[arg(long, default_value_t = RPC_DEFAULT_MESSAGE_CAPACITY_PER_CONN)]
	pub rpc_message_buffer_capacity_per_connection: u32,

	/// Disable RPC batch requests
	#[arg(long, alias = "rpc_no_batch_requests", conflicts_with_all = &["rpc_max_batch_request_len"])]
	pub rpc_disable_batch_requests: bool,

	/// Limit the max length per RPC batch request
	#[arg(long, conflicts_with_all = &["rpc_disable_batch_requests"], value_name = "LEN")]
	pub rpc_max_batch_request_len: Option<u32>,

	/// Specify browser *origins* allowed to access the HTTP & WS RPC servers.
	///
	/// A comma-separated list of origins (protocol://domain or special `null`
	/// value). Value of `all` will disable origin validation. Default is to
	/// allow localhost and <https://polkadot.js.org> origins. When running in
	/// `--dev` mode the default is to allow all origins.
	#[arg(long, value_name = "ORIGINS")]
	pub rpc_cors: Option<Cors>,

	/// The human-readable name for this node.
	///
	/// It's used as network node name.
	#[arg(long, value_name = "NAME")]
	pub name: Option<String>,

	#[allow(missing_docs)]
	#[clap(flatten)]
	pub telemetry_params: TelemetryParams,

	#[allow(missing_docs)]
	#[clap(flatten)]
	pub prometheus_params: PrometheusParams,

	#[allow(missing_docs)]
	#[clap(flatten)]
	pub runtime_params: RuntimeParams,

	#[allow(missing_docs)]
	#[clap(flatten)]
	pub offchain_worker_params: OffchainWorkerParams,

	#[allow(missing_docs)]
	#[clap(flatten)]
	pub shared_params: SharedParams,

	#[allow(missing_docs)]
	#[clap(flatten)]
	pub import_params: ImportParams,

	#[allow(missing_docs)]
	#[clap(flatten)]
	pub network_params: NetworkParams,

	#[allow(missing_docs)]
	#[clap(flatten)]
	pub pool_config: TransactionPoolParams,

	#[allow(missing_docs)]
	#[clap(flatten)]
	pub keystore_params: KeystoreParams,

	/// Shortcut for `--name Alice --validator`.
	///
	/// Session keys for `Alice` are added to keystore.
	#[arg(long, conflicts_with_all = &["bob", "charlie", "dave", "eve", "ferdie", "one", "two"])]
	pub alice: bool,

	/// Shortcut for `--name Bob --validator`.
	///
	/// Session keys for `Bob` are added to keystore.
	#[arg(long, conflicts_with_all = &["alice", "charlie", "dave", "eve", "ferdie", "one", "two"])]
	pub bob: bool,

	/// Shortcut for `--name Charlie --validator`.
	///
	/// Session keys for `Charlie` are added to keystore.
	#[arg(long, conflicts_with_all = &["alice", "bob", "dave", "eve", "ferdie", "one", "two"])]
	pub charlie: bool,

	/// Shortcut for `--name Dave --validator`.
	///
	/// Session keys for `Dave` are added to keystore.
	#[arg(long, conflicts_with_all = &["alice", "bob", "charlie", "eve", "ferdie", "one", "two"])]
	pub dave: bool,

	/// Shortcut for `--name Eve --validator`.
	///
	/// Session keys for `Eve` are added to keystore.
	#[arg(long, conflicts_with_all = &["alice", "bob", "charlie", "dave", "ferdie", "one", "two"])]
	pub eve: bool,

	/// Shortcut for `--name Ferdie --validator`.
	///
	/// Session keys for `Ferdie` are added to keystore.
	#[arg(long, conflicts_with_all = &["alice", "bob", "charlie", "dave", "eve", "one", "two"])]
	pub ferdie: bool,

	/// Shortcut for `--name One --validator`.
	///
	/// Session keys for `One` are added to keystore.
	#[arg(long, conflicts_with_all = &["alice", "bob", "charlie", "dave", "eve", "ferdie", "two"])]
	pub one: bool,

	/// Shortcut for `--name Two --validator`.
	///
	/// Session keys for `Two` are added to keystore.
	#[arg(long, conflicts_with_all = &["alice", "bob", "charlie", "dave", "eve", "ferdie", "one"])]
	pub two: bool,

	/// Enable authoring even when offline.
	#[arg(long)]
	pub force_authoring: bool,

	/// Run a temporary node.
	///
	/// A temporary directory will be created to store the configuration and will be deleted
	/// at the end of the process.
	///
	/// Note: the directory is random per process execution. This directory is used as base path
	/// which includes: database, node key and keystore.
	///
	/// When `--dev` is given and no explicit `--base-path`, this option is implied.
	#[arg(long, conflicts_with = "base_path")]
	pub tmp: bool,
}

impl RunCmd {
	/// Get the `Sr25519Keyring` matching one of the flag.
	pub fn get_keyring(&self) -> Option<sp_keyring::Sr25519Keyring> {
		use sp_keyring::Sr25519Keyring::*;

		if self.alice {
			Some(Alice)
		} else if self.bob {
			Some(Bob)
		} else if self.charlie {
			Some(Charlie)
		} else if self.dave {
			Some(Dave)
		} else if self.eve {
			Some(Eve)
		} else if self.ferdie {
			Some(Ferdie)
		} else if self.one {
			Some(One)
		} else if self.two {
			Some(Two)
		} else {
			None
		}
	}
}

impl CliConfiguration for RunCmd {
	fn shared_params(&self) -> &SharedParams {
		&self.shared_params
	}

	fn import_params(&self) -> Option<&ImportParams> {
		Some(&self.import_params)
	}

	fn network_params(&self) -> Option<&NetworkParams> {
		Some(&self.network_params)
	}

	fn keystore_params(&self) -> Option<&KeystoreParams> {
		Some(&self.keystore_params)
	}

	fn offchain_worker_params(&self) -> Option<&OffchainWorkerParams> {
		Some(&self.offchain_worker_params)
	}

	fn node_name(&self) -> Result<String> {
		let name: String = match (self.name.as_ref(), self.get_keyring()) {
			(Some(name), _) => name.to_string(),
			(_, Some(keyring)) => keyring.to_string(),
			(None, None) => crate::generate_node_name(),
		};

		is_node_name_valid(&name).map_err(|msg| {
			Error::Input(format!(
				"Invalid node name '{}'. Reason: {}. If unsure, use none.",
				name, msg
			))
		})?;

		Ok(name)
	}

	fn dev_key_seed(&self, is_dev: bool) -> Result<Option<String>> {
		Ok(self.get_keyring().map(|a| format!("//{}", a)).or_else(|| {
			if is_dev {
				Some("//Alice".into())
			} else {
				None
			}
		}))
	}

	fn telemetry_endpoints(
		&self,
		chain_spec: &Box<dyn ChainSpec>,
	) -> Result<Option<TelemetryEndpoints>> {
		let params = &self.telemetry_params;
		Ok(if params.no_telemetry {
			None
		} else if !params.telemetry_endpoints.is_empty() {
			Some(
				TelemetryEndpoints::new(params.telemetry_endpoints.clone())
					.map_err(|e| e.to_string())?,
			)
		} else {
			chain_spec.telemetry_endpoints().clone()
		})
	}

	fn role(&self, is_dev: bool) -> Result<Role> {
		let keyring = self.get_keyring();
		let is_authority = self.validator || is_dev || keyring.is_some();

		Ok(if is_authority { Role::Authority } else { Role::Full })
	}

	fn force_authoring(&self) -> Result<bool> {
		// Imply forced authoring on --dev
		Ok(self.shared_params.dev || self.force_authoring)
	}

	fn prometheus_config(
		&self,
		default_listen_port: u16,
		chain_spec: &Box<dyn ChainSpec>,
	) -> Result<Option<PrometheusConfig>> {
		Ok(self
			.prometheus_params
			.prometheus_config(default_listen_port, chain_spec.id().to_string()))
	}

	fn disable_grandpa(&self) -> Result<bool> {
		Ok(self.no_grandpa)
	}

	fn rpc_max_connections(&self) -> Result<u32> {
		Ok(self.rpc_max_connections)
	}

	fn rpc_cors(&self, is_dev: bool) -> Result<Option<Vec<String>>> {
		Ok(self
			.rpc_cors
			.clone()
			.unwrap_or_else(|| {
				if is_dev {
					log::warn!("Running in --dev mode, RPC CORS has been disabled.");
					Cors::All
				} else {
					Cors::List(vec![
						"http://localhost:*".into(),
						"http://127.0.0.1:*".into(),
						"https://localhost:*".into(),
						"https://127.0.0.1:*".into(),
						"https://polkadot.js.org".into(),
					])
				}
			})
			.into())
	}

	fn rpc_addr(&self, default_listen_port: u16) -> Result<Option<Vec<RpcEndpoint>>> {
		if !self.experimental_rpc_endpoint.is_empty() {
			for endpoint in &self.experimental_rpc_endpoint {
				// Technically, `0.0.0.0` isn't a public IP address, but it's a way to listen on
				// all interfaces. Thus, we consider it as a public endpoint and warn about it.
				if endpoint.rpc_methods == RpcMethods::Unsafe && endpoint.is_global() ||
					endpoint.listen_addr.ip().is_unspecified()
				{
					log::warn!(
						"It isn't safe to expose RPC publicly without a proxy server that filters \
						 available set of RPC methods."
					);
				}
			}

			return Ok(Some(self.experimental_rpc_endpoint.clone()));
		}

		let (ipv4, ipv6) = rpc_interface(
			self.rpc_external,
			self.unsafe_rpc_external,
			self.rpc_methods,
			self.validator,
		)?;

		let cors = self.rpc_cors(self.is_dev()?)?;
		let port = self.rpc_port.unwrap_or(default_listen_port);

		Ok(Some(vec![
			RpcEndpoint {
				batch_config: self.rpc_batch_config()?,
				max_connections: self.rpc_max_connections,
				listen_addr: SocketAddr::new(std::net::IpAddr::V4(ipv4), port),
				rpc_methods: self.rpc_methods,
				rate_limit: self.rpc_rate_limit,
				rate_limit_trust_proxy_headers: self.rpc_rate_limit_trust_proxy_headers,
				rate_limit_whitelisted_ips: self.rpc_rate_limit_whitelisted_ips.clone(),
				max_payload_in_mb: self.rpc_max_request_size,
				max_payload_out_mb: self.rpc_max_response_size,
				max_subscriptions_per_connection: self.rpc_max_subscriptions_per_connection,
				max_buffer_capacity_per_connection: self.rpc_message_buffer_capacity_per_connection,
				cors: cors.clone(),
				retry_random_port: true,
				is_optional: false,
			},
			RpcEndpoint {
				batch_config: self.rpc_batch_config()?,
				max_connections: self.rpc_max_connections,
				listen_addr: SocketAddr::new(std::net::IpAddr::V6(ipv6), port),
				rpc_methods: self.rpc_methods,
				rate_limit: self.rpc_rate_limit,
				rate_limit_trust_proxy_headers: self.rpc_rate_limit_trust_proxy_headers,
				rate_limit_whitelisted_ips: self.rpc_rate_limit_whitelisted_ips.clone(),
				max_payload_in_mb: self.rpc_max_request_size,
				max_payload_out_mb: self.rpc_max_response_size,
				max_subscriptions_per_connection: self.rpc_max_subscriptions_per_connection,
				max_buffer_capacity_per_connection: self.rpc_message_buffer_capacity_per_connection,
				cors: cors.clone(),
				retry_random_port: true,
				is_optional: true,
			},
		]))
	}

	fn rpc_methods(&self) -> Result<sc_service::config::RpcMethods> {
		Ok(self.rpc_methods.into())
	}

	fn rpc_max_request_size(&self) -> Result<u32> {
		Ok(self.rpc_max_request_size)
	}

	fn rpc_max_response_size(&self) -> Result<u32> {
		Ok(self.rpc_max_response_size)
	}

	fn rpc_max_subscriptions_per_connection(&self) -> Result<u32> {
		Ok(self.rpc_max_subscriptions_per_connection)
	}

	fn rpc_buffer_capacity_per_connection(&self) -> Result<u32> {
		Ok(self.rpc_message_buffer_capacity_per_connection)
	}

	fn rpc_batch_config(&self) -> Result<RpcBatchRequestConfig> {
		let cfg = if self.rpc_disable_batch_requests {
			RpcBatchRequestConfig::Disabled
		} else if let Some(l) = self.rpc_max_batch_request_len {
			RpcBatchRequestConfig::Limit(l)
		} else {
			RpcBatchRequestConfig::Unlimited
		};

		Ok(cfg)
	}

	fn rpc_rate_limit(&self) -> Result<Option<NonZeroU32>> {
		Ok(self.rpc_rate_limit)
	}

	fn rpc_rate_limit_whitelisted_ips(&self) -> Result<Vec<IpNetwork>> {
		Ok(self.rpc_rate_limit_whitelisted_ips.clone())
	}

	fn rpc_rate_limit_trust_proxy_headers(&self) -> Result<bool> {
		Ok(self.rpc_rate_limit_trust_proxy_headers)
	}

	fn transaction_pool(&self, is_dev: bool) -> Result<TransactionPoolOptions> {
		Ok(self.pool_config.transaction_pool(is_dev))
	}

	fn max_runtime_instances(&self) -> Result<Option<usize>> {
		Ok(Some(self.runtime_params.max_runtime_instances))
	}

	fn runtime_cache_size(&self) -> Result<u8> {
		Ok(self.runtime_params.runtime_cache_size)
	}

	fn base_path(&self) -> Result<Option<BasePath>> {
		Ok(if self.tmp {
			Some(BasePath::new_temp_dir()?)
		} else {
			match self.shared_params().base_path()? {
				Some(r) => Some(r),
				// If `dev` is enabled, we use the temp base path.
				None if self.shared_params().is_dev() => Some(BasePath::new_temp_dir()?),
				None => None,
			}
		})
	}
}

/// Check whether a node name is considered as valid.
pub fn is_node_name_valid(_name: &str) -> std::result::Result<(), &str> {
	let name = _name.to_string();

	if name.is_empty() {
		return Err("Node name cannot be empty")
	}

	if name.chars().count() >= crate::NODE_NAME_MAX_LENGTH {
		return Err("Node name too long")
	}

	let invalid_chars = r"[\\.@]";
	let re = Regex::new(invalid_chars).unwrap();
	if re.is_match(&name) {
		return Err("Node name should not contain invalid chars such as '.' and '@'")
	}

	let invalid_patterns = r"^https?:";
	let re = Regex::new(invalid_patterns).unwrap();
	if re.is_match(&name) {
		return Err("Node name should not contain urls")
	}

	Ok(())
}

fn rpc_interface(
	is_external: bool,
	is_unsafe_external: bool,
	rpc_methods: RpcMethods,
	is_validator: bool,
) -> Result<(Ipv4Addr, Ipv6Addr)> {
	if is_external && is_validator && rpc_methods != RpcMethods::Unsafe {
		return Err(Error::Input(
			"--rpc-external option shouldn't be used if the node is running as \
			 a validator. Use `--unsafe-rpc-external` or `--rpc-methods=unsafe` if you understand \
			 the risks. See the options description for more information."
				.to_owned(),
		))
	}

	if is_external || is_unsafe_external {
		if rpc_methods == RpcMethods::Unsafe {
			log::warn!(
				"It isn't safe to expose RPC publicly without a proxy server that filters \
				 available set of RPC methods."
			);
		}

		Ok((Ipv4Addr::UNSPECIFIED, Ipv6Addr::UNSPECIFIED))
	} else {
		Ok((Ipv4Addr::LOCALHOST, Ipv6Addr::LOCALHOST))
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn tests_node_name_good() {
		assert!(is_node_name_valid("short name").is_ok());
		assert!(is_node_name_valid("www").is_ok());
		assert!(is_node_name_valid("aawww").is_ok());
		assert!(is_node_name_valid("wwwaa").is_ok());
		assert!(is_node_name_valid("www aa").is_ok());
	}

	#[test]
	fn tests_node_name_bad() {
		assert!(is_node_name_valid("").is_err());
		assert!(is_node_name_valid(
			"very very long names are really not very cool for the ui at all, really they're not"
		)
		.is_err());
		assert!(is_node_name_valid("Dots.not.Ok").is_err());
		// NOTE: the urls below don't include a domain otherwise
		// they'd get filtered for including a `.`
		assert!(is_node_name_valid("http://visitme").is_err());
		assert!(is_node_name_valid("http:/visitme").is_err());
		assert!(is_node_name_valid("http:visitme").is_err());
		assert!(is_node_name_valid("https://visitme").is_err());
		assert!(is_node_name_valid("https:/visitme").is_err());
		assert!(is_node_name_valid("https:visitme").is_err());
		assert!(is_node_name_valid("www.visit.me").is_err());
		assert!(is_node_name_valid("www.visit").is_err());
		assert!(is_node_name_valid("hello\\world").is_err());
		assert!(is_node_name_valid("visit.www").is_err());
		assert!(is_node_name_valid("email@domain").is_err());
	}
}
