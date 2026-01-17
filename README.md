# MQTT Auth Proxy

A simple sidecar proxy that can handle JWT authentication for clients that do not support it.

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
4. Install the broker: `kubectl apply -f example/config-security.yaml -f example/config-broker.yaml -f example/broker.yaml`
5. Deploy the test client: `kubectl apply -f example/mqtt-client.yaml`
6. Test direct broker connection: `k exec -it $(k get pod -l app.kubernetes.io/name=mqtt-client -o name) -c mqttui -- mqttui pub -b mqtts://hivemq.default.svc.cluster.local:8883 --insecure some/topc "hello world"`. This will fail as the client does not authenticate itself
7. Test connection via the sidecar proxy: `k exec -it $(k get pod -l app.kubernetes.io/name=mqtt-client -o name) -c mqttui -- mqttui pub -b mqtt://localhost:1883 some/topc "hello world"`. This will succeed
