# Request

Dispatches given request to the required service.

## Details

```
POST /api/v1/request
Authorization: Bearer ${YOUR JWT}
Gateway-Local-Tracking-Id: ${YOUR_TRACKING_ID}
```
### Headers
`Gateway-Local-Tracking-Id` is optional header allowing to specify tracking id.

### Parameters

Name        | Type      | Default    | Description
----------- | --------- | ---------- | -----------
me          | AgentId   | _required_ | Your AgentId
destination | AccountId | _required_ | Target service name
method      | String    | _required_ | Method you wish to call on the target service
payload     | String    | _required_ | Request body

## Response

You should get a response as described in specific service documentation.
