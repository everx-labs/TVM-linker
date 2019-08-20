pragma solidity ^0.5.0;

contract Test28 {

    uint[] arr;

    function main(uint required_len) public payable returns (uint) {
        arr[0] = 2;
        arr[1] = 3;
        arr[2] = 3;
        arr[3] = 3;
        arr[4] = 3;
        arr[5] = 3;
        arr[6] = 3;
        arr.length = required_len;
        return arr.length;
    }
}