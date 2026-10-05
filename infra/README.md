# Setup

- Create `<whatever>.auto.tfvars` with `project = <your_project>`
- `tofu apply -auto-approve`
- Connect via `gcloud compute ssh workhorse --zone europe-west3-c --project <your_project>`
