import * as cdk from 'aws-cdk-lib';
import { Construct } from 'constructs';
import { VpcStack } from './vpc-stack';
import { ComputeStack } from './compute-stack';
import { AlbStack, SandboxConfig } from './alb-stack';

export class InfraStack extends cdk.Stack {
  constructor(scope: Construct, id: string, props?: cdk.StackProps) {
    super(scope, id, props);

    // 1. VPC
    const vpcStack = new VpcStack(this, 'VpcLayer');

    // 2. Compute
    const computeStack = new ComputeStack(this, 'ComputeLayer', {
      vpc: vpcStack.vpc,
    });

    // 3. Define Sandboxes
    const sandboxes: SandboxConfig[] = [
      {
        name: 'sandbox1',
        frontendPort: 8080,
        keycloakPort: 8180,
        appHost: 'sb1.example.com',
        authHost: 'auth-sb1.example.com',
      },
      {
        name: 'sandbox2',
        frontendPort: 8081,
        keycloakPort: 8181,
        appHost: 'sb2.example.com',
        authHost: 'auth-sb2.example.com',
      },
    ];

    // 4. ALB & Routing
    new AlbStack(this, 'AlbLayer', {
      vpc: vpcStack.vpc,
      instance: computeStack.instance,
      sandboxes: sandboxes,
    });
  }
}
