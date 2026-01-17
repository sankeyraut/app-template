import * as cdk from 'aws-cdk-lib';
import * as ec2 from 'aws-cdk-lib/aws-ec2';
import * as elbv2 from 'aws-cdk-lib/aws-elasticloadbalancingv2';
import * as targets from 'aws-cdk-lib/aws-elasticloadbalancingv2-targets';
import { Construct } from 'constructs';

export interface SandboxConfig {
    name: string;
    frontendPort: number;
    keycloakPort: number;
    appHost: string;
    authHost: string;
}

interface AlbStackProps extends cdk.NestedStackProps {
    vpc: ec2.IVpc;
    instance: ec2.Instance;
    sandboxes: SandboxConfig[];
}

export class AlbStack extends cdk.NestedStack {
    constructor(scope: Construct, id: string, props: AlbStackProps) {
        super(scope, id, props);

        const alb = new elbv2.ApplicationLoadBalancer(this, 'AppAlb', {
            vpc: props.vpc,
            internetFacing: true,
        });

        const listener = alb.addListener('HttpListener', {
            port: 80, // Using 80 for simplicity. In prod, use 443 with ACM cert.
            open: true,
        });

        // Default action (if no host matches)
        listener.addAction('DefaultAction', {
            action: elbv2.ListenerAction.fixedResponse(404, {
                contentType: 'text/plain',
                messageBody: 'Environment not found.',
            }),
        });

        props.sandboxes.forEach((sb) => {
            // 1. Frontend Routing
            const frontendTarget = new elbv2.ApplicationTargetGroup(this, `${sb.name}FrontendTarget`, {
                vpc: props.vpc,
                port: sb.frontendPort,
                protocol: elbv2.ApplicationProtocol.HTTP,
                targets: [new targets.InstanceTarget(props.instance, sb.frontendPort)],
                healthCheck: { path: '/', port: sb.frontendPort.toString() },
            });

            listener.addTargetGroups(`${sb.name}FrontendRule`, {
                priority: 10 + (props.sandboxes.indexOf(sb) * 2),
                conditions: [elbv2.ListenerCondition.hostHeaders([sb.appHost])],
                targetGroups: [frontendTarget],
            });

            // 2. Keycloak Routing
            const authTarget = new elbv2.ApplicationTargetGroup(this, `${sb.name}AuthTarget`, {
                vpc: props.vpc,
                port: sb.keycloakPort,
                protocol: elbv2.ApplicationProtocol.HTTP,
                targets: [new targets.InstanceTarget(props.instance, sb.keycloakPort)],
                healthCheck: { path: '/health/live', port: sb.keycloakPort.toString() },
            });

            listener.addTargetGroups(`${sb.name}AuthRule`, {
                priority: 11 + (props.sandboxes.indexOf(sb) * 2),
                conditions: [elbv2.ListenerCondition.hostHeaders([sb.authHost])],
                targetGroups: [authTarget],
            });

            // Allow traffic from ALB to EC2 on these ports
            props.instance.connections.allowFrom(alb, ec2.Port.tcp(sb.frontendPort), `Allow HTTP for ${sb.name} frontend`);
            props.instance.connections.allowFrom(alb, ec2.Port.tcp(sb.keycloakPort), `Allow HTTP for ${sb.name} Keycloak`);
        });

        new cdk.CfnOutput(this, 'AlbDnsName', {
            value: alb.loadBalancerDnsName,
        });
    }
}
