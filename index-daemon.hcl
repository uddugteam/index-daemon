job "quazar-idx-daemon" {
  group "quazar-idx-daemon" {
    network {
      port "ws" {
        to = 8000
      }
    }

    service {
      name = "quazar-idx-daemon"
      port = "ws"
    }

    task "quazar-idx-daemon" {
      driver = "docker"
      kill_timeout = "20s"

      config {
        image = "andskur/index-daemon:latest" // must be fulfilled!
        ports = ["idx"]
      }

      resources {
        cpu    = 2048
        memory = 1024
      }

      env {
        APP__SERVICE_CONFIG__WS       = "1"
        APP__SERVICE_CONFIG__WS_PORT  = "8000"
      }
    }
  }
}

