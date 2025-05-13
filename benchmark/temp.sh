#!/bin/bash

# List of regions
regions=("us-east-1" "us-east-2" "us-west-1" "us-west-2" "ca-central-1" "eu-west-1" "ap-southeast-1" "ap-northeast-1")

# Loop through each region
for region in "${regions[@]}"; do
  echo "Fetching latest AMI for region: $region"
  aws ec2 describe-images \
    --owners amazon \
    --filters "Name=name,Values=amzn2-ami-hvm-*-x86_64-gp2" "Name=state,Values=available" \
    --query "Images[*].[ImageId,CreationDate]" \
    --region "$region" \
    --output text | sort -k2 -r | head -n 1
done