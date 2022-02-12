const cdk = require("@aws-cdk/core");
const {
  GithubActionsIdentityProvider,
  GithubActionsRole,
} = require("aws-cdk-github-oidc");
const dynamo = require("@aws-cdk/aws-dynamodb");
const { CfnOutput } = require("@aws-cdk/core");
const iam = require("@aws-cdk/aws-iam");

class CdkStack extends cdk.Stack {
  /**
   *
   * @param {cdk.Construct} scope
   * @param {string} id
   * @param {cdk.StackProps=} props
   */
  constructor(scope, id, props) {
    super(scope, id, props);

    const provider = GithubActionsIdentityProvider.fromAccount(
      this,
      "GitHubProvider"
    );

    const statsTable = new dynamo.Table(this, "ESMCheckerDynamoStatsTable", {
      billingMode: dynamo.BillingMode.PAY_PER_REQUEST,
      partitionKey: {
        name: "year_month",
        type: dynamo.AttributeType.STRING,
      },
      sortKey: {
        name: "timestamp",
        type: dynamo.AttributeType.STRING,
      },
      removalPolicy: cdk.RemovalPolicy.DESTROY,
    });

    cdk.Tags.of(statsTable).add("Project", "esm-checker");

    const packageTable = new dynamo.Table(
      this,
      "ESMCheckerDynamoPackageTable",
      {
        billingMode: dynamo.BillingMode.PAY_PER_REQUEST,
        partitionKey: {
          name: "package_name",
          type: dynamo.AttributeType.STRING,
        },
        removalPolicy: cdk.RemovalPolicy.DESTROY,
      }
    );

    cdk.Tags.of(packageTable).add("Project", "esm-checker");

    const auditTable = new dynamo.Table(this, "ESMCheckerDynamoAuditTable", {
      billingMode: dynamo.BillingMode.PAY_PER_REQUEST,
      partitionKey: {
        name: "timestamp",
        type: dynamo.AttributeType.STRING,
      },
      sortKey: {
        name: "package_name_id",
        type: dynamo.AttributeType.STRING,
      },
      removalPolicy: cdk.RemovalPolicy.DESTROY,
    });

    cdk.Tags.of(auditTable).add("Project", "esm-checker");

    auditTable.addGlobalSecondaryIndex({
      indexName: "packageIndex",
      partitionKey: {
        name: "package_name",
        type: dynamo.AttributeType.STRING,
      },
      sortKey: {
        name: "timestamp",
        type: dynamo.AttributeType.STRING,
      },
    });

    const dynamoRole = new GithubActionsRole(this, "ESMCheckerDynamoRole", {
      provider,
      owner: "lannonbr",
      repo: "esm-checker",
      filter: "ref:refs/heads/main",
    });

    cdk.Tags.of(dynamoRole).add("Project", "esm-checker");

    const siteReadRole = new GithubActionsRole(this, "ESMCheckerSiteReadRole", {
      provider,
      owner: "lannonbr",
      repo: "esm-checker-site",
      filter: "ref:refs/heads/main",
    });

    cdk.Tags.of(siteReadRole).add("Project", "esm-checker");

    statsTable.grantReadWriteData(dynamoRole);
    packageTable.grantReadWriteData(dynamoRole);
    auditTable.grantReadWriteData(dynamoRole);

    statsTable.grantReadData(siteReadRole);
    auditTable.grantReadData(siteReadRole);

    const netlifyUser = new iam.User(this, "ESMCheckerNetlifyUser", {
      userName: "esm-checker-netlify",
    });

    packageTable.grantReadData(netlifyUser);

    const netlifyAccessKey = new iam.CfnAccessKey(
      this,
      "ESMCheckerNetlifyAccessKey",
      {
        userName: netlifyUser.userName,
      }
    );

    new CfnOutput(this, "ESMCheckerNetlifyToken", {
      value: netlifyAccessKey.ref,
    });
    new CfnOutput(this, "ESMCheckerNetlifySecret", {
      value: netlifyAccessKey.attrSecretAccessKey,
    });

    const tableName = new CfnOutput(this, "ESMCheckerDynamoStatsTableName", {
      value: statsTable.tableName,
    });
  }
}

module.exports = { CdkStack };
