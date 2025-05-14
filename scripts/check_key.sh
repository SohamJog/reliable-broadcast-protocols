for region in us-east-1 us-east-2 us-west-1 us-west-2 ca-central-1 eu-west-1 ap-southeast-1 ap-northeast-1; do
  echo -n "$region: "
  aws ec2 describe-key-pairs \
    --region "$region" \
    --key-names add_rbc_benchmark \
    --query "KeyPairs[*].KeyName" \
    --output text
done
