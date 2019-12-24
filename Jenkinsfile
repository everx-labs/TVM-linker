@Library('infrastructure-jenkins-shared-library@master')_

G_gitcred = "LaninSSHgit"
G_promoted_branch = 'origin/master'
G_buildstatus = 'NotSet'
G_teststatus = 'NotSet'

pipeline {
    parameters {
        booleanParam (
            defaultValue: false,
            description: 'Promote image built to be used as latest',
            name : 'FORCE_PROMOTE_LATEST'
        )
    }
    agent {
        node {label 'master'}
    }
    options {
        buildDiscarder logRotator(artifactDaysToKeepStr: '', artifactNumToKeepStr: '', daysToKeepStr: '', numToKeepStr: '10')
        disableConcurrentBuilds()
        parallelsAlwaysFailFast()
    }
    triggers {
        upstream(
            upstreamProjects: 'Node/ton-labs-abi/master',
            threshold: hudson.model.Result.SUCCESS
        )
    }
    stages {
        stage('Build') {
            failFast true
            parallel {
                stage ('Build docker image') {
                    steps {
                        script {
                            G_dockerimage = "tonlabs/tvm_linker:${GIT_COMMIT}"
                            sshagent (credentials: [G_gitcred]) {
                                withEnv(["DOCKER_BUILDKIT=1", "BUILD_INFO=${env.BUILD_TAG}:${GIT_COMMIT}"]) {
                                    app_image = docker.build(
                                        "${G_dockerimage}",
                                        '--label "git-commit=${GIT_COMMIT}" --ssh default .'
                                    )
                                }
                            }
                        }
                    }
                }
                stage('Build linux') {
                    when { 
                        branch 'master'
                    }
                    agent {
                        docker {
                            image "atomxy/build-rust:20191223"
                        }
                    }
                    steps {
                        script {
                            withAWS(credentials: 'CI_bucket_writer', region: 'eu-central-1') {
                                identity = awsIdentity()
                                s3Download bucket: 'sdkbinaries.tonlabs.io', file: 'tvm_linker.json', force: true, path: 'tvm_linker.json'
                            }
                            dir('tvm_linker') {
                                sh """
                                    cargo update
                                    cargo build --release
                                    chmod a+x tvm_linker/target/release 
                                """
                            }
                            sh 'node gzip.js tvm_linker/target/release'
                            withAWS(credentials: 'CI_bucket_writer', region: 'eu-central-1') {
                                identity = awsIdentity()
                                s3Upload \
                                    bucket: 'sdkbinaries.tonlabs.io', \
                                    includePathPattern:'*.gz', path: 'tmp_linker', \
                                    workingDir:'.'
                                s3Upload \
                                    bucket: 'sdkbinaries.tonlabs.io', \
                                    includePathPattern:'tvm_linker.json', path: 'tmp_linker', \
                                    workingDir:'.'
                            }
                        }
                    }
                    post {
						cleanup {script{cleanWs notFailBuild: true}}
					}
                }
                stage('Build darwin') {
                    when { 
                        branch 'master'
                    }
                    agent {
                        label 'ios'
                    }
                    steps {
                        script {
                            withAWS(credentials: 'CI_bucket_writer', region: 'eu-central-1') {
                                identity = awsIdentity()
                                s3Download bucket: 'sdkbinaries.tonlabs.io', file: 'tvm_linker.json', force: true, path: 'tvm_linker.json'
                            }
                            dir('tvm_linker') {
                                sh """
                                    cargo update
                                    cargo build --release
                                    chmod a+x tvm_linker/target/release 
                                """
                            }
                            sh 'node gzip.js tvm_linker/target/release'
                            withAWS(credentials: 'CI_bucket_writer', region: 'eu-central-1') {
                                identity = awsIdentity()
                                s3Upload \
                                    bucket: 'sdkbinaries.tonlabs.io', \
                                    includePathPattern:'*.gz', path: 'tmp_linker', \
                                    workingDir:'.'
                                s3Upload \
                                    bucket: 'sdkbinaries.tonlabs.io', \
                                    includePathPattern:'tvm_linker.json', path: 'tmp_linker', \
                                    workingDir:'.'
                            }
                        }
                    }
                    post {
						cleanup {script{cleanWs notFailBuild: true}}
					}
                }
                stage('Build windows') {
                    when { 
                        branch 'master'
                    }
                    agent {
                        label 'Win'
                    }
                    steps {
                        script {
                            withAWS(credentials: 'CI_bucket_writer', region: 'eu-central-1') {
                                identity = awsIdentity()
                                s3Download bucket: 'sdkbinaries.tonlabs.io', file: 'tvm_linker.json', force: true, path: 'tvm_linker.json'
                            }
                            dir('tvm_linker') {
                                bat """
                                    cargo update
                                    cargo build --release
                                """
                            }
                            sh 'node gzip.js tvm_linker\\target\\release'
                            withAWS(credentials: 'CI_bucket_writer', region: 'eu-central-1') {
                                identity = awsIdentity()
                                s3Upload \
                                    bucket: 'sdkbinaries.tonlabs.io', \
                                    includePathPattern:'*.gz', path: 'tmp_linker', \
                                    workingDir:'.'
                                s3Upload \
                                    bucket: 'sdkbinaries.tonlabs.io', \
                                    includePathPattern:'tvm_linker.json', path: 'tmp_linker', \
                                    workingDir:'.'
                            }
                        }
                    }
                    post {
						cleanup {script{cleanWs notFailBuild: true}}
					}
                }
            }
        }
        stage ('Test') {
            agent {
                docker {
                    image "${G_dockerimage}"
                    alwaysPull false
                    args '-u root'
                }
            }
            steps {
                script {
                    sh 'apk add python'
                    sh '/usr/bin/tvm_linker --version'
                    sh 'cd tvm_linker && python test_suite.py --linker-path=/usr/bin/tvm_linker'
                }
            }
        }
	    stage('Push docker-image') {
            steps {
                script {
                    docker.withRegistry('', 'dockerhubLanin') {
                        app_image.push()
                    }
                }
            }
            post {
                failure {script{G_buildstatus = "failure"}}
            }
        }
        stage('Test in compiler-kit') {
            steps {
                script {
                    G_dockerimage = "tonlabs/tvm_linker:${GIT_COMMIT}"
                    def params = [
                      [$class: 'StringParameterValue', name: 'dockerimage_tvm_linker', value: "${G_dockerimage}"]
                    ]
                    build job : "Infrastructure/compilers/master", parameters : params
                }
            }
            post {
                success {
                    script{
                        G_buildstatus = "success"
                    }
                }
                failure {script{G_buildstatus = "failure"}}
            }
        }
        stage ('Tag as latest') {
            when {
                expression {
                    // GIT_BRANCH = 'origin/' + sh(returnStdout: true, script: 'git rev-parse --abbrev-ref HEAD').trim()
                    GIT_BRANCH = "origin/${BRANCH_NAME}"
                    return GIT_BRANCH == G_promoted_branch || params.FORCE_PROMOTE_LATEST
                }
            }
            steps {
                script {
                    docker.withRegistry('', 'dockerhubLanin') {
                        docker.image("${G_dockerimage}").push('latest')
                    }
                }
            }
        }
    }
    post {
        success {
            node ('master') {
                script {
                    withAWS(credentials: 'CI_bucket_writer', region: 'eu-central-1') {
                        identity = awsIdentity()
                        list = s3FindFiles(bucket: 'sdkbinaries.tonlabs.io', path: 'tmp_linker/', glob: '*')
                        for (def file : list) {
                            s3Copy fromBucket: 'sdkbinaries.tonlabs.io', fromPath: "tmp_linker/${file.path}", toBucket: 'sdkbinaries.tonlabs.io', toPath: "${file.path}"
                        
                        }
                        s3Delete bucket: 'sdkbinaries.tonlabs.io', path: 'tmp_linker/'
                    }
                }
            }
        }
        failure {
            node ('master') {
                script {
                    withAWS(credentials: 'CI_bucket_writer', region: 'eu-central-1') {
                        identity = awsIdentity()
                        s3Delete bucket: 'sdkbinaries.tonlabs.io', path: 'tmp_linker/'
                    }
                }
            }
        }
        always {
            notifyTeam(
                buildstatus: G_buildstatus
            ) 
        }
    }
}
