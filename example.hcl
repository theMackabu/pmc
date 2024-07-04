process "test_prod" {
  script = "node ./test.js"

  env {
    NODE_ENV = "production"
  }
  
  watch {
    path = "./test.js"
  }
}

process "test" {
  script = "node ./test.js"
}