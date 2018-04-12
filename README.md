# http-gateway

[![Build Status](https://travis-ci.org/netology-group/http-gateway.svg?branch=master)](https://travis-ci.org/netology-group/http-gateway)

[Documentation](http://http-gateway.docs.netology-group.services)

# API

## rpc_call

### URI
```
/rpc_call
```
### Description
This method allows you to send HTTP messages to the MQTT broker in the JSON-RPC
format and receive the reply in the same format. Note that the body of the
request must include the id property.

![Alt text](docs/diagrams/rpc_call_sequence?raw=true "Title")

### Headers

For a more detailed description of the parameters, see
* http://docs.oasis-open.org/mqtt/mqtt/v3.1.1/os/mqtt-v3.1.1-os.html#_Toc398718031
* http://docs.oasis-open.org/mqtt/mqtt/v3.1.1/os/mqtt-v3.1.1-os.html#_Toc398718106
* http://docs.oasis-open.org/mqtt/mqtt/v3.1.1/os/mqtt-v3.1.1-os.html#_Toc398718099

Name | Default | Description
-----|---------|------------
x-mqtt-client-id|required|Each Client connecting to the Server has a unique ClientId 
x-mqtt-username|required|It can be used by the Server for authentication and authorization
x-mqtt-password|required|Used with login
x-mqtt-publish-topic|required|valid MQTT topic name
x-mqtt-publish-qos|required|Quality of Service. Can take the values 0, 1 and 2
x-mqtt-publish-retain|0|Retained messages are useful where publishers send state messages on an irregular basis. A new subscriber will receive the most recent state. Can take the values 0 and 1
x-mqtt-subscribe-topic|required|valid MQTT topic name
x-mqtt-subscribe-qos|required|Quality of Service. Can take the values 0, 1 and 2
x-mqtt-last-will-topic|optional|valid MQTT topic name
x-mqtt-last-will-message|optional|This message is published if the connection is broken

### Example
```bash
curl -v -d '{"jsonrpc":"2.0","method":"agent.create","params":[{"id":"da64655c-825b-45b3-ad35-60c43d36aab7"}],"id":"qwerty"}' \
-H "Content-Type: application/json" \
-H "x-mqtt-client-id: faf7a1d6-3a49-45ad-8685-bbec10849f3d.da64655c-825b-45b3-ad35-60c43d36aab7" \
-H "x-mqtt-publish-topic: agents/da64655c-825b-45b3-ad35-60c43d36aab7/out/signals.netology-group.services/api/v1" \
-H "x-mqtt-subscribe-topic: agents/da64655c-825b-45b3-ad35-60c43d36aab7/in/signals.netology-group.services/api/v1" \
-H "x-mqtt-username: usr" \
-H "x-mqtt-password: pwd" \
-H "x-mqtt-subscribe-qos: 1" \
-H "x-mqtt-publish-qos: 1" \
-H "x-mqtt-last-will-topic: lwt" \
-H "x-mqtt-last-will-message: lwm" \
-X POST localhost/rpc_call
```

# Configuration variables

Name | Default | Description
-----|---------|------------
MQTT_BROKER_URL|localhost:1883|The addres of the broker (like test.mosquitto.org:1883)
LISTEN_PORT|80|Port where the application will listen for HTTP requests

