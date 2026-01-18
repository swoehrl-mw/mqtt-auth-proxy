# MQTT Auth Proxy

A simple sidecar proxy that can handle JWT authentication for clients that do not support it.

Normal authentication for MQTT brokers is done using either username/password or TLS client certificates. But both of these mechanisms require credentials (the password or certificate) to be managed and provisioned with the client. You need secure (and automated) mechanisms to provide the credentials and to rotate them. Always with the risk of them being compromised or stolen.
For workloads in Kubernetes there is a better way to do secure authentication: [Pod Workload Identiy](https://blog.sebastianwoehrl.name/blog/2024-10-k3s-workload-identity/) allows Kubernetes Pods to use tokens provided by the Kubernetes Control Plane, acting as an [OpenID Connect Identity Provider](https://openid.net/developers/how-connect-works/), to authenticate with external systems.

The widely used HiveMQ MQTT Broker supports this with [JWT auth](https://docs.hivemq.com/hivemq-enterprise-security-extension/latest/ese.html#jwt-clients). But most clients do not. This proxy aims to bridge that gap. It is intended to be run as a sidecar in the Kubernetes Pod of an MQTT client that cannot use JWT auth. The client talks MQTT without auth over a local connection with the sidecar, and it takes care of performing the JWT auth and then proxying the rest of the connection. 

## Example

This example demonstrates the usage of the proxy to connect an MQTT client with a [HiveMQ cluster](https://docs.hivemq.com/hivemq/latest/user-guide/index.html) that is configured with [JWT auth](https://docs.hivemq.com/hivemq-enterprise-security-extension/latest/ese.html#jwt-clients) against the internal Kubernetes Identity Provider. 

You need a Kubernetes cluster with anonymous auth enabled. For [K3s](https://k3s.io), create/edit `/etc/rancher/k3s/config.yaml` and add:

```yaml
kube-apiserver-arg:
  - anonymous-auth=true
```

Afterwards restart k3s (`sudo systemctl restart k3s`). You also need to allow anyonmous access to the jwks endpoint:

```bash
kubectl apply -f - <<EOF
apiVersion: rbac.authorization.k8s.io/v1
kind: ClusterRoleBinding
metadata:
  name: service-account-issuer-discovery-unauthenticated
roleRef:
  apiGroup: rbac.authorization.k8s.io
  kind: ClusterRole
  name: system:service-account-issuer-discovery
subjects:
- apiGroup: rbac.authorization.k8s.io
  kind: Group
  name: system:unauthenticated
EOF
```

1. Install the [HiveMQ Platform Operator](https://docs.hivemq.com/hivemq-platform-operator/introduction.html): `helm repo add hivemq https://hivemq.github.io/helm-charts && helm repo update && helm install hivemq-platform-operator hivemq/hivemq-platform-operator`
2. Install [cert-manager](https://cert-manager.io/docs/installation/kubectl/): `kubectl apply -f https://github.com/cert-manager/cert-manager/releases/download/v1.19.2/cert-manager.yaml`
3. Create the needed certificates: `kubectl apply -f example/tls-certs.yaml`
4. Install the broker: `kubectl apply -f example/config-security.yaml -f example/config-broker.yaml -f example/broker.yaml` (without providing a license an automatic trial license valid for 5 hours is used)
5. Deploy the test client: `kubectl apply -f example/mqtt-client.yaml`. It contains [mqttui](https://github.com/EdJoPaTo/mqttui) along with the proxy sidecar
6. Test direct broker connection: `k exec -it $(k get pod -l app.kubernetes.io/name=mqtt-client -o name) -c mqttui -- mqttui pub -b mqtts://hivemq.default.svc.cluster.local:8883 --insecure some/topc "hello world"`. This will fail as the client does not authenticate itself
7. Test connection via the sidecar proxy: `k exec -it $(k get pod -l app.kubernetes.io/name=mqtt-client -o name) -c mqttui -- mqttui pub -b mqtt://localhost:1883 some/topc "hello world"`. This will succeed

## Development

This project is written in Rust and requires a recent stable [Rust/Cargo](https://rust-lang.org/learn/get-started/) version.

It contains a number of unit and one e2e test for verification of functionality. Run them with `cargo test`.
