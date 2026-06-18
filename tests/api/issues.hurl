# Gitpage Issues/Labels API Tests
# Variables: auth_token, repo_id

POST {{host}}/api/repos/{{repo_id}}/labels
Authorization: Bearer {{auth_token}}
Content-Type: application/json
{
    "name": "bug",
    "color": "d73a4a"
}
HTTP 201

GET {{host}}/api/repos/{{repo_id}}/labels
Authorization: Bearer {{auth_token}}
HTTP 200
[Asserts]
jsonpath "$.labels[0].name" == "bug"

POST {{host}}/api/repos/{{repo_id}}/issues
Authorization: Bearer {{auth_token}}
Content-Type: application/json
{
    "title": "Test Bug",
    "body": "Found a bug"
}
HTTP 201
[Captures]
issue_number: jsonpath "$.issue.issue.number"
[Asserts]
jsonpath "$.issue.issue.number" == 1
jsonpath "$.issue.issue.title" == "Test Bug"

GET {{host}}/api/repos/{{repo_id}}/issues
Authorization: Bearer {{auth_token}}
HTTP 200
[Asserts]
jsonpath "$.issues" isCollection

GET {{host}}/api/repos/{{repo_id}}/issues/{{issue_number}}
Authorization: Bearer {{auth_token}}
HTTP 200
[Asserts]
jsonpath "$.issue.issue.title" == "Test Bug"

POST {{host}}/api/repos/{{repo_id}}/issues/{{issue_number}}/comments
Authorization: Bearer {{auth_token}}
Content-Type: application/json
{
    "body": "I can reproduce this"
}
HTTP 201

GET {{host}}/api/repos/{{repo_id}}/issues/{{issue_number}}/comments
Authorization: Bearer {{auth_token}}
HTTP 200
[Asserts]
jsonpath "$.comments[0].body" == "I can reproduce this"

PUT {{host}}/api/repos/{{repo_id}}/issues/{{issue_number}}
Authorization: Bearer {{auth_token}}
Content-Type: application/json
{
    "state": "closed"
}
HTTP 200

GET {{host}}/api/repos/{{repo_id}}/issues/{{issue_number}}
Authorization: Bearer {{auth_token}}
HTTP 200
[Asserts]
jsonpath "$.issue.issue.state" == "closed"
