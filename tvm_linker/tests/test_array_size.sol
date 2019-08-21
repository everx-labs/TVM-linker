pragma solidity ^0.5.0;

contract Test28 {

    function tvm_logstr(bytes32 logstr) private {}
    function tvm_init_storage() private {}

    uint[] arr;

    function main(uint16 required_len) public payable returns (uint) {
        tvm_init_storage();
        arr[0] = 2;
        arr[1] = 3;
        arr[2] = 3;
        arr[3] = 3;
        arr[4] = 3;
        arr[5] = 3;
        arr[6] = 3;
        arr.length = required_len;
        require(arr[6] == 0);
        arr.length = required_len + 10;
        require(arr[required_len + 10 - 1] == 0);
        return arr.length;
    }
}