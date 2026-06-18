# Gitpage Org API Tests

POST {{host}}/api/orgs
Authorization: Bearer {{auth_token}}
Content-Type: application/json
{
    "name": "hurl-org-{{timestamp}}",
    "display_name": "Hurl Org",
    "description": "test org"
}
HTTP 201
[Captures]
org_name: jsonpath "$.org.name"
[Asserts]
jsonpath "$.org.id" exists
jsonpath "$.org.name" == "hurl-org-{{timestamp}}"

POST {{host}}/api/orgs
Authorization: Bearer {{auth_token}}
Content-Type: application/json
{
    "name": "hurl-org-{{timestamp}}"
}
HTTP 409

GET {{host}}/api/orgs/{{org_name}}
HTTP 200
[Asserts]
jsonpath "$.org.name" == "hurl-org-{{timestamp}}"

GET {{host}}/api/orgs
Authorization: Bearer {{auth_token}}
HTTP 200
[Asserts]
jsonpath "$.orgs" isCollection
