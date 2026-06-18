# Gitpage Content API Tests
# Variables: owner_name, repo_name (from git push)

GET {{host}}/api/{{owner_name}}/{{repo_name}}/tree?branch=main
HTTP 200
[Asserts]
jsonpath "$.entries" isCollection
jsonpath "$.branch" == "main"

GET {{host}}/api/{{owner_name}}/{{repo_name}}/readme?branch=main
HTTP 200
[Asserts]
jsonpath "$.has_readme" == true
jsonpath "$.rendered" isString

GET {{host}}/api/{{owner_name}}/{{repo_name}}/commits/main
HTTP 200
[Asserts]
jsonpath "$.commits" isCollection
