variable "project" {
  type = string
}

variable "region" {
  type    = string
  default = "europe-west3"
}

variable "vpc-name" {
  type    = string
  default = "quadtree-bench-vpc"
}

variable "zone" {
  type    = string
  default = "europe-west3-c"
}
