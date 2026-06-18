# Gitpage Auth API Tests
# Usage: hurl --variable host=http://localhost:8080 --file-root . auth.hurl

POST {{host}}/api/auth/register
Content-Type: application/json
{
    "username": "hurl-{{timestamp}}",
    "email": "hurl-{{timestamp}}@test.com",
    "password": "pass123"
}
HTTP 201
[Captures]
auth_token: jsonpath "$.token"
[Asserts]
jsonpath "$.token" isString
jsonpath "$.user.id" exists
jsonpath "$.user.username" == "hurl-{{timestamp}}"

POST {{host}}/api/auth/register
Content-Type: application/json
{
    "username": "hurl-{{timestamp}}",
    "email": "dup-{{timestamp}}@test.com",
    "password": "pass123"
}
HTTP 409

POST {{host}}/api/auth/login
Content-Type: application/json
{
    "username": "hurl-{{timestamp}}",
    "password": "pass123"
}
HTTP 200
[Asserts]
jsonpath "$.token" isString
jsonpath "$.user.username" == "hurl-{{timestamp}}"

POST {{host}}/api/auth/login
Content-Type: application/json
{
    "username": "hurl-{{timestamp}}",
    "password": "wrongpassword"
}
HTTP 401

GET {{host}}/api/auth/me
HTTP 401

GET {{host}}/api/auth/me
Authorization: Bearer {{auth_token}}
HTTP 200
[Asserts]
jsonpath "$.user.id" exists
jsonpath "$.user.username" == "hurl-{{timestamp}}"
