# Gitpage Settings API Tests
# Variables: auth_token, repo_id, username

POST {{host}}/api/user/tokens
Authorization: Bearer {{auth_token}}
Content-Type: application/json
{
    "name": "hurl-token",
    "scopes": "repo:read"
}
HTTP 201
[Asserts]
jsonpath "$.raw_token" matches "^gpt_.*"

POST {{host}}/api/repos/{{repo_id}}/collaborators
Authorization: Bearer {{auth_token}}
Content-Type: application/json
{
    "username": "{{username}}",
    "permission": "read"
}
HTTP 200

GET {{host}}/api/repos/{{repo_id}}/collaborators
Authorization: Bearer {{auth_token}}
HTTP 200
[Asserts]
jsonpath "$.collaborators[0].username" exists

POST {{host}}/api/repos/{{repo_id}}/secrets
Authorization: Bearer {{auth_token}}
Content-Type: application/json
{
    "name": "MY_SECRET",
    "value": "supersecret"
}
HTTP 201

GET {{host}}/api/repos/{{repo_id}}/secrets
Authorization: Bearer {{auth_token}}
HTTP 200
[Asserts]
jsonpath "$.secrets[0].name" == "MY_SECRET"

POST {{host}}/api/repos/{{repo_id}}/branch-protections
Authorization: Bearer {{auth_token}}
Content-Type: application/json
{
    "pattern": "main",
    "require_pull_request": true,
    "require_approvals": 1
}
HTTP 201

GET {{host}}/api/repos/{{repo_id}}/branch-protections
Authorization: Bearer {{auth_token}}
HTTP 200
[Asserts]
jsonpath "$.branch_protections[0].pattern" == "main"
