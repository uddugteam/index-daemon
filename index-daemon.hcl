job "index-daemon" {
  group "idx-daemon" {
    network {
      port "idx" {
        to = 8000
      }
    }
    task "idx-daemon" {
      driver = "docker"
      kill_timeout = "20s"

      config {
        image = "misnaged/index-daemon:latest" // must be fulfilled!
        ports = ["idx"]
        auth {
          username = ""
          password = ""
        }
      }

      resources {
        cpu    = 2048
        memory = 2048
      }
    }
  }
}

