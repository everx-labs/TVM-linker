@Library('infrastructure-jenkins-shared-library@master')_

G_rust_image = "rust:1.40"
G_gitcred = "LaninSSHgit"
G_docker_creds = 'dockerhubLanin'
G_promoted_branch = 'origin/master'
G_buildstatus = 'NotSet'
G_teststatus = 'NotSet'
G_docker_src_image = null
G_docker_pub_image = null

pipeline {
    parameters {

        booleanParam (
            defaultValue: false,
            description: 'Promote image built to be used as latest',
            name : 'FORCE_PROMOTE_LATEST'
        )
        string(
            name:'dockerImage_ton_types',
            defaultValue: 'tonlabs/ton-types:latest',
            description: 'Existing ton-types image name'
        )
        string(
            name:'dockerImage_ton_block',
            defaultValue: 'tonlabs/ton-block:latest',
            description: 'Existing ton-block image name'
        )
        string(
            name:'dockerImage_ton_vm',
            defaultValue: 'tonlabs/ton-vm:latest',
            description: 'Existing ton-vm image name'
        )
        string(
            name:'dockerImage_ton_labs_abi',
            defaultValue: 'tonlabs/ton-labs-abi:latest',
            description: 'Existing ton-labs-abi image name'
        )
        string(
            name:'dockerImage_tvm_linker',
            defaultValue: '',
            description: 'Expected ton-executor image name'
        )
        string(
            name:'ton_sdk_branch',
            defaultValue: 'master',
            description: 'ton-sdk branch for upstairs test'
        )
    }
    agent {
        node {label 'master'}
    }
    tools {nodejs "Node12.8.0"}
    options {
        buildDiscarder logRotator(artifactDaysToKeepStr: '', artifactNumToKeepStr: '', daysToKeepStr: '', numToKeepStr: '10')
        disableConcurrentBuilds()
        parallelsAlwaysFailFast()
    }
    stages {
        stage('Switch to file source') {
            steps {
                script {
                    sh """
(cat tvm_linker/Cargo.toml | \
sed 's/ton_types = .*/ton_types = { path = \"\\/tonlabs\\/ton-types\" }/g' | \
sed 's/ton_block = .*/ton_block = { path = \"\\/tonlabs\\/ton-block\" }/g' | \
sed 's/ton_abi = .*/ton_abi = { path = \"\\/tonlabs\\/ton-labs-abi\" }/g' | \
sed 's/ton_vm = .*/ton_vm = { path = \"\\/tonlabs\\/ton-vm\", default-features = false }/g') > ./tvm_linker/tmp.toml
rm ./tvm_linker/Cargo.toml
mv ./tvm_linker/tmp.toml ./tvm_linker/Cargo.toml
                    """
                }
            }
        }
        stage('Build sources image') {
            steps {
                script {
                    G_docker_src_image = "tonlabs/tvm_linker:src-${GIT_COMMIT}"
                    docker.withRegistry('', G_docker_creds) {
                        sshagent (credentials: [G_gitcred]) {
                            withEnv(["DOCKER_BUILDKIT=1", "BUILD_INFO=src-${env.BUILD_TAG}:${GIT_COMMIT}"]) {
                                app_src_image = docker.build(
                                    "${G_docker_src_image}",
                                    "--label \"git-commit=\${GIT_COMMIT}\" --target tvm-linker-src ."
                                )
                            }
                        }
                        docker.image("${G_docker_src_image}").push()
                    }
                }
            }
        }
        stage('Prepare sources for agents') {
            agent {
                dockerfile {
                    additionalBuildArgs "--target linker-src"
                }
            }
            steps {
                script {
                    sh """
                        zip -9 -r linker-src.zip /tonlabs/*
                        chown jenkins:jenkins linker-src.zip
                    """
                    stash includes: '**/linker-src.zip', name: 'linker-src'
                }
            }
        }
        stage('Build') {
            failFast true
            parallel {
                stage ('Build docker image') {
                    steps {
                        script {
                            G_docker_pub_image = "tonlabs/tvm_linker:${GIT_COMMIT}"
                            docker.withRegistry('', G_docker_creds) {
                                sshagent (credentials: [G_gitcred]) {
                                    withEnv(["DOCKER_BUILDKIT=1", "BUILD_INFO=${env.BUILD_TAG}:${GIT_COMMIT}"]) {
                                        app_image = docker.build(
                                            "${G_docker_pub_image}",
                                            "--label \"git-commit=\${GIT_COMMIT}\" --ssh default " + 
                                            "--build-arg \"RUST_IMAGE=${G_rust_image}\" " + 
                                            "--build-arg \"TON_TYPES_IMAGE=${params.dockerImage_ton_types}\" " +
                                            "--build-arg \"TON_BLOCK_IMAGE=${params.dockerImage_ton_block}\" " + 
                                            "--build-arg \"TON_VM_IMAGE=${params.dockerImage_ton_vm}\" " + 
                                            "--build-arg \"TON_LABS_ABI_IMAGE=${params.dockerImage_ton_labs_abi}\" " + 
                                            "."
                                        )
                                    }
                                }
                            }
                        }
                    }
                }
                stage('Build linux') {
                    /*when { 
                        branch 'master'
                    }*/
                    agent {
                        dockerfile {
                            additionalBuildArgs "--target build-ton-compiler"
                        }
                    }
                    steps {
                        script {
                            withAWS(credentials: 'CI_bucket_writer', region: 'eu-central-1') {
                                identity = awsIdentity()
                                s3Download bucket: 'sdkbinaries.tonlabs.io', file: 'tvm_linker.json', force: true, path: 'tvm_linker.json'
                            }
                            sh 'node gzip.js ../../../../app/tvm_linker'
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
                    /*when { 
                        branch 'master'
                    }*/
                    agent {
                        label 'ios'
                    }
                    steps {
                        script {
                            withAWS(credentials: 'CI_bucket_writer', region: 'eu-central-1') {
                                identity = awsIdentity()
                                s3Download bucket: 'sdkbinaries.tonlabs.io', file: 'tvm_linker.json', force: true, path: 'tvm_linker.json'
                            }
                            def C_PATH = sh (script: 'pwd', returnStdout: true).trim()
                            
                            unstash 'linker-src'
                            sh """
                                unzip linker-src.zip
                                node pathFix.js tonlabs/ton-block/Cargo.toml \"{ path = \\\"/tonlabs/\" \"{ path = \\\"${C_PATH}/tonlabs/\"
                                node pathFix.js tonlabs/ton-vm/Cargo.toml \"{ path = \\\"/tonlabs/\" \"{ path = \\\"${C_PATH}/tonlabs/\"
                                node pathFix.js tonlabs/ton-labs-abi/Cargo.toml \"{ path = \\\"/tonlabs/\" \"{ path = \\\"${C_PATH}/tonlabs/\"
                                node pathFix.js tonlabs/tvm_linker/Cargo.toml \"{ path = \\\"/tonlabs/\" \"{ path = \\\"${C_PATH}/tonlabs/\"
                            """
                            dir('tonlabs') {
                                dir('tvm_linker') {
                                    sh """
                                        cargo update
                                        cargo build --release
                                        chmod a+x target/release/tvm_linker
                                    """
                                }
                            }
                            sh 'node gzip.js tonlabs/tvm_linker/target/release/tvm_linker'
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
                    /*when { 
                        branch 'master'
                    }*/
                    agent {
                        label 'Win'
                    }
                    steps {
                        script {
                            def C_PATH = bat (script: '@echo off && echo %cd%', returnStdout: true).trim()
                            echo "${C_PATH}"
                            withAWS(credentials: 'CI_bucket_writer', region: 'eu-central-1') {
                                identity = awsIdentity()
                                s3Download bucket: 'sdkbinaries.tonlabs.io', file: 'tvm_linker.json', force: true, path: 'tvm_linker.json'
                            }
                            unstash 'linker-src'
                            bat """
                                unzip linker-src.zip
                                node pathFix.js tonlabs\\ton-block\\Cargo.toml '{ path = \"/tonlabs/' '{ path = \"${C_PATH}\\tonlabs\\'
                                node pathFix.js tonlabs\\ton-vm\\Cargo.toml '{ path = \"/tonlabs/' '{ path = \"${C_PATH}\\tonlabs\\''
                                node pathFix.js tonlabs\\ton-labs-abi\\Cargo.toml '{ path = \"/tonlabs/' '{ path = \"${C_PATH}\\tonlabs\\'
                                node pathFix.js tonlabs\\tvm_linker\\Cargo.toml '{ path = \"/tonlabs/' '{ path = \"${C_PATH}\\tonlabs\\'
                            """
                            dir('tonlabs') {
                                dir('tvm_linker') {
                                    bat """
                                        cargo update
                                        cargo build --release
                                        chmod a+x target/release/tvm_linker
                                    """
                                }
                            }

                            bat "node gzip.js tonlabs\\tvm_linker\\target\\release\\tvm_linker.exe"
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
                    image "${G_docker_pub_image}"
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
                    docker.withRegistry('', G_docker_creds) {
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
                    G_docker_pub_image = "tonlabs/tvm_linker:${GIT_COMMIT}"
                    def params = [
                      [$class: 'StringParameterValue', name: 'dockerimage_tvm_linker', value: "${G_docker_pub_image}"]
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
                    docker.withRegistry('', G_docker_creds) {
                        docker.image("${G_docker_pub_image}").push('latest')
                        docker.image("${G_docker_src_image}").push('src-latest')
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
