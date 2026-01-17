import * as cdk from 'aws-cdk-lib';
import * as ec2 from 'aws-cdk-lib/aws-ec2';
import * as iam from 'aws-cdk-lib/aws-iam';
import { Construct } from 'constructs';

interface ComputeStackProps extends cdk.NestedStackProps {
    vpc: ec2.IVpc;
}

export class ComputeStack extends cdk.NestedStack {
    public readonly instance: ec2.Instance;

    constructor(scope: Construct, id: string, props: ComputeStackProps) {
        super(scope, id, props);

        // IAM Role for EC2
        const role = new iam.Role(this, 'AppInstanceRole', {
            assumedBy: new iam.ServicePrincipal('ec2.amazonaws.com'),
            managedPolicies: [
                iam.ManagedPolicy.fromAwsManagedPolicyName('AmazonSSMManagedInstanceCore'), // For terminal access without SSH
            ],
        });

        // Security Group for EC2
        const securityGroup = new ec2.SecurityGroup(this, 'AppSecurityGroup', {
            vpc: props.vpc,
            allowAllOutbound: true,
            description: 'Security group for App Template EC2 instance',
        });

        // Instance
        this.instance = new ec2.Instance(this, 'AppInstance', {
            vpc: props.vpc,
            instanceType: ec2.InstanceType.of(ec2.InstanceClass.T3, ec2.InstanceSize.MEDIUM),
            machineImage: ec2.MachineImage.latestAmazonLinux2023(),
            role: role,
            securityGroup: securityGroup,
            vpcSubnets: { subnetType: ec2.SubnetType.PRIVATE_WITH_EGRESS },
        });

        // UserData: Install Docker and Git
        this.instance.addUserData(
            'yum update -y',
            'yum install -y docker git',
            'systemctl start docker',
            'systemctl enable docker',
            'usermod -a -G docker ec2-user',
            'curl -L https://github.com/docker/compose/releases/latest/download/docker-compose-$(uname -s)-$(uname -m) -o /usr/local/bin/docker-compose',
            'chmod +x /usr/local/bin/docker-compose'
        );
    }
}
