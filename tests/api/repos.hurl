# Gitpage Repo API Tests
# Variables: auth_token, timestamp, repo_id (pre-created repo)

GET {{host}}/api/repos/{{repo_id}}
Authorization: Bearer {{auth_token}}
HTTP 200
[Asserts]
jsonpath "$.repo.id" exists
jsonpath "$.repo.name" exists

GET {{host}}/api/repos
Authorization: Bearer {{auth_token}}
HTTP 200
[Asserts]
jsonpath "$.repos" isCollection

PUT {{host}}/api/repos/{{repo_id}}
Authorization: Bearer {{auth_token}}
Content-Type: application/json
{
    "name": "hurl-repo-renamed",
    "description": "renamed",
    "is_private": true
}
HTTP 200

# Revert name
PUT {{host}}/api/repos/{{repo_id}}
Authorization: Bearer {{auth_token}}
Content-Type: application/json
{
    "name": "hurl-repo-{{timestamp}}",
    "is_private": false
}
HTTP 200

GET {{host}}/api/repos/99999999
Authorization: Bearer {{auth_token}}
HTTP 404
