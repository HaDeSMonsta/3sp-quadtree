resource "google_compute_instance" "bench-vm" {
  boot_disk {
    device_name = "bencher"

    initialize_params {
      image = "projects/debian-cloud/global/images/debian-13-trixie-v20260921"
      size  = 10
    }
  }

  name         = "workhorse"
  machine_type = "c3-standard-8"
  zone         = var.zone

  network_interface {
    subnetwork = google_compute_subnetwork.sub.id

    access_config {}
  }

  metadata = {
    enable-osconfig = "TRUE"
    startup-script = templatefile("./setup.sh", {})
  }
}
