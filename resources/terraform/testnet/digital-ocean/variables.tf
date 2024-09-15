variable "droplet_ssh_keys" {
  type = list(number)
  default = [
    37243057, # Benno Zeeman
    38313409, # Roland Sherwin
    36971688, # David Irvine
    19315097, # Stephen Coyle
    29201567, # Josh Wilson
    30643816, # Anselme Grumbach
    30113222, # Qi Ma
    42022675, # Shu
    42317962, # Mazzi
    30878672, # Chris O'Neil
    31216015, # QA
    34183228, # GH Actions Automation
    38596814, # sn-testnet-workflows automation
    29586082
  ]
}

variable "node_droplet_size" {
  description = "The size of the droplet for generic nodes VMs"
}

variable "bootstrap_droplet_size" {
  description = "The size of the droplet for bootstrap nodes VMs"
}

variable "uploader_droplet_size" {
  description = "The size of the droplet for uploader VMs"
}

variable "build_machine_size" {
  default = "s-8vcpu-16gb"
}

variable "auditor_droplet_image_id" {
  default = "156295663"
}

variable "build_droplet_image_id" {
  default = "165140612"
}

variable "bootstrap_droplet_image_id" {
  description = "The ID of the bootstrap node droplet image. Varies per environment type."
}

variable "node_droplet_image_id" {
  description = "The ID of the node droplet image. Varies per environment type."
}

variable "uploader_droplet_image_id" {
  description = "The ID of the uploader droplet image. Varies per environment type."
}

variable "region" {
  default = "lon1"
}

variable "genesis_vm_count" {
  default     = 1
  description = "Set to 1 or 0 to control whether there is a genesis node"
}

variable "auditor_vm_count" {
  default     = 1
  description = "The number of auditor droplets"
}

variable "bootstrap_node_vm_count" {
  default     = 2
  description = "The number of droplets to launch for bootstrap nodes"
}

variable "node_vm_count" {
  default     = 10
  description = "The number of droplets to launch for nodes"
}

variable "uploader_vm_count" {
  default     = 2
  description = "The number of droplets to launch for uploaders"
}

variable "use_custom_bin" {
  type        = bool
  default     = false
  description = "A boolean to enable use of a custom bin"
}
